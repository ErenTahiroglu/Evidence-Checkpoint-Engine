// evaluator.rs — Conservative evidence evaluator comparing GitHub observations.

use super::client::{GitHubClientError, GitHubProvider};
use super::models::GitHubArtifact;
use super::policy::{PolicyStatus, TrustedCheckPolicy};
use crate::verifier::assessment::validate_and_wrap;
use crate::verifier::error::VerifierError;
use crate::verifier::schema::SchemaStore;
use crate::verifier::VerificationOutput;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeclaredArtifactSpec {
    pub artifact_name: String,
    pub artifact_id: Option<u64>,
    pub expected_digest: Option<String>,
    pub verify_bytes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvelopeVerificationContext {
    pub envelope_digest: String,
    pub is_digest_verified: bool,
    pub is_envelope: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetEvidenceRequest {
    pub owner: String,
    pub repo: String,
    pub expected_repo_id: String,
    pub commit_sha: String,
    pub run_id: u64,
    pub run_attempt: u32,
    pub expected_workflow_path: Option<String>,
    pub checkpoint_id: Option<String>,
    pub declared_phase: Option<String>,
    pub declared_producer: Option<String>,
    pub declared_artifacts: Vec<DeclaredArtifactSpec>,
}

pub fn evaluate_github_evidence(
    provider: &dyn GitHubProvider,
    target: &TargetEvidenceRequest,
    policy: &TrustedCheckPolicy,
    schema_dir: &Path,
    envelope_ctx: Option<&EnvelopeVerificationContext>,
) -> Result<VerificationOutput, VerifierError> {
    let store = SchemaStore::load(schema_dir)?;
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let assessment_id = format!("ece-eval-{}", Uuid::new_v4());

    let mut checks_performed = Vec::new();
    let mut checks_omitted = vec![
        "Independent human authorization (out of scope for CI adapter)".to_string(),
        "Physical device execution (hosted CI runners only, not bare-metal hardware)".to_string(),
        "Cryptographic GPG commit signature verification (commit is unsigned)".to_string(),
    ];
    let mut blocking_reasons = Vec::new();
    let mut warnings = Vec::new();

    let (
        evidence_integrity_status,
        digest_match,
        integrity_details,
        digest_verified,
        schema_conformant,
    ) = if let Some(ctx) = envelope_ctx {
        if ctx.is_envelope && ctx.is_digest_verified {
            checks_performed.push(format!(
                "RFC 8785 envelope digest verification ({})",
                ctx.envelope_digest
            ));
            (
                "VALID",
                true,
                format!(
                    "Envelope RFC 8785 digest {} verified; GitHub REST API evidence evaluated",
                    ctx.envelope_digest
                ),
                true,
                true,
            )
        } else {
            checks_omitted.push(
                "Envelope digest verification (bypassed for bare payload input)".to_string(),
            );
            warnings.push(
                "BARE_PAYLOAD_EVIDENCE: Envelope digest verification was omitted; \
                 evaluating bare checkpoint payload"
                    .to_string(),
            );
            (
                "NOT_EVALUATED",
                false,
                "Envelope digest check omitted (bare payload input)".to_string(),
                false,
                true,
            )
        }
    } else {
        checks_omitted.push(
            "Envelope digest verification (omitted in standalone run assessment)".to_string(),
        );
        warnings.push(
            "STANDALONE_RUN_ASSESSMENT: Envelope digest verification was omitted; \
             CI facts evaluated directly from GitHub REST API"
                .to_string(),
        );
        (
            "NOT_EVALUATED",
            false,
            "Envelope digest check omitted (run assessment mode)".to_string(),
            false,
            false,
        )
    };

    let mut sha_verified = false;
    let mut repo_id_matched = false;
    let mut ci_status = "UNVERIFIED_RUN";
    let expected_jobs_count = policy.required_jobs.len();
    let mut unresolved_jobs = Vec::new();
    let mut expired_references = Vec::new();

    // 1. Fetch Repository Identity
    checks_performed.push("GitHub REST API repository identity lookup".to_string());
    let repo_res = provider.get_repository(&target.owner, &target.repo);

    let repo = match repo_res {
        Ok(r) => r,
        Err(e) => {
            return build_api_failure_assessment(
                &store,
                target,
                &assessment_id,
                &timestamp,
                "repository",
                &e,
                checks_performed,
                checks_omitted,
            );
        }
    };

    // 2. Strict Repository ID Binding (Numeric string match)
    checks_performed.push("Provider repository numeric ID binding verification".to_string());
    if repo.id.to_string() != target.expected_repo_id {
        blocking_reasons.push(format!(
            "Repository numeric ID mismatch: declared '{}' but provider returned '{}'",
            target.expected_repo_id, repo.id
        ));
    } else {
        repo_id_matched = true;
    }

    // 3. Commit Existence Check
    checks_performed.push("GitHub REST API target commit existence check".to_string());
    let commit_res = provider.get_commit(&target.owner, &target.repo, &target.commit_sha);
    match commit_res {
        Ok(c) => {
            if c.sha.to_lowercase() == target.commit_sha.to_lowercase() {
                sha_verified = true;
            } else {
                blocking_reasons.push(format!(
                    "Commit SHA mismatch: requested '{}' but provider returned '{}'",
                    target.commit_sha, c.sha
                ));
            }
        }
        Err(e) => {
            blocking_reasons.push(format!(
                "Target commit '{}' could not be verified in repository: {}",
                target.commit_sha, e
            ));
        }
    }

    // 4. Workflow Run Binding Check
    checks_performed.push("GitHub REST API workflow run attempt binding".to_string());
    let run_res = provider.get_workflow_run(&target.owner, &target.repo, target.run_id);
    let run = match run_res {
        Ok(r) => r,
        Err(e) => {
            return build_api_failure_assessment(
                &store,
                target,
                &assessment_id,
                &timestamp,
                "workflow_run",
                &e,
                checks_performed,
                checks_omitted,
            );
        }
    };

    // Verify run belongs to this repository
    if let Some(ref run_repo) = run.repository {
        if run_repo.id != repo.id {
            blocking_reasons.push(format!(
                "Workflow run {} belongs to repo ID {} but expected repo ID is {}",
                run.id, run_repo.id, repo.id
            ));
        }
    }

    // Verify run attempt semantics
    if target.run_attempt == 0 {
        ci_status = "ATTEMPT_MISMATCH";
        blocking_reasons.push(
            "Invalid run attempt number: requested attempt is 0 (attempts are 1-based)".to_string(),
        );
    } else if target.run_attempt > run.run_attempt {
        ci_status = "ATTEMPT_MISMATCH";
        blocking_reasons.push(format!(
            "Run attempt mismatch: requested attempt {} exceeds latest provider run attempt {}",
            target.run_attempt, run.run_attempt
        ));
    } else if target.run_attempt < run.run_attempt {
        warnings.push(format!(
            "Evaluating historical run attempt {} (latest run attempt is {})",
            target.run_attempt, run.run_attempt
        ));
    }

    // Verify head_sha matches target commit
    if run.head_sha.to_lowercase() != target.commit_sha.to_lowercase() {
        sha_verified = false;
        blocking_reasons.push(format!(
            "Workflow run head_sha '{}' does not match target commit SHA '{}'",
            run.head_sha, target.commit_sha
        ));
    }

    // Verify workflow path if declared
    if let Some(ref expected_path) = target.expected_workflow_path {
        if let Some(ref actual_path) = run.path {
            if actual_path != expected_path {
                blocking_reasons.push(format!(
                    "Workflow definition path mismatch: declared '{}' but provider returned '{}'",
                    expected_path, actual_path
                ));
            }
        }
    }

    // Verify target scope against policy
    if let Err(scope_err) =
        policy.validate_target_scope(&target.expected_repo_id, run.path.as_deref())
    {
        blocking_reasons.push(scope_err);
    }

    // Verify run overall status and conclusion
    if run.status != "completed" {
        ci_status = "UNVERIFIED_RUN";
        blocking_reasons.push(format!(
            "Workflow run {} is not completed (current status='{}')",
            run.id, run.status
        ));
    } else if run.conclusion.as_deref() != Some("success") {
        ci_status = "FAILED";
        blocking_reasons.push(format!(
            "Workflow run {} overall conclusion was '{:?}'",
            run.id, run.conclusion
        ));
    }

    // 5. Run-Attempt Specific Jobs Verification
    checks_performed
        .push("GitHub REST API exact attempt jobs matching against trusted policy".to_string());
    let jobs_res = provider.get_attempt_jobs(
        &target.owner,
        &target.repo,
        target.run_id,
        target.run_attempt,
    );
    let jobs = match jobs_res {
        Ok(jobs) => jobs,
        Err(e) => {
            return build_api_failure_assessment(
                &store,
                target,
                &assessment_id,
                &timestamp,
                "attempt_jobs",
                &e,
                checks_performed,
                checks_omitted,
            );
        }
    };
    let observed_jobs_count = jobs.len();
    let eval = policy.evaluate_jobs(&jobs);

    match eval.status {
        PolicyStatus::VerifiedPass => {
            if blocking_reasons.is_empty() {
                ci_status = "VERIFIED_PASS";
            }
        }
        PolicyStatus::MissingJobs => {
            ci_status = "FAILED";
            unresolved_jobs = eval.missing_jobs.clone();
            blocking_reasons.push(format!(
                "Required CI jobs missing from run attempt: {:?}",
                eval.missing_jobs
            ));
        }
        PolicyStatus::Failed => {
            ci_status = "FAILED";
            unresolved_jobs = eval.failed_jobs.clone();
            blocking_reasons.push(format!(
                "Required CI jobs failed in run attempt: {:?}",
                eval.failed_jobs
            ));
        }
        PolicyStatus::IncompleteSkipped => {
            ci_status = "INCOMPLETE_SKIPPED";
            unresolved_jobs = eval.skipped_or_cancelled_jobs.clone();
            blocking_reasons.push(format!(
                "Required CI jobs skipped or cancelled in run attempt: {:?}",
                eval.skipped_or_cancelled_jobs
            ));
        }
        PolicyStatus::InProgress => {
            ci_status = "UNVERIFIED_RUN";
            unresolved_jobs = eval.in_progress_jobs.clone();
            blocking_reasons.push(format!(
                "Required CI jobs still in progress: {:?}",
                eval.in_progress_jobs
            ));
        }
        PolicyStatus::AmbiguousDuplicated => {
            ci_status = "FAILED";
            unresolved_jobs = eval.conflicting_jobs.clone();
            blocking_reasons.push(format!(
                "Ambiguous duplicate executions observed in run attempt: {:?}",
                eval.conflicting_jobs
            ));
        }
        PolicyStatus::UntrustedPolicy => {
            ci_status = "UNVERIFIED_RUN";
            blocking_reasons.push(format!(
                "Trusted check policy is invalid or untrusted: {:?}",
                eval.details
            ));
        }
    }

    // 6. Artifact Verification
    checks_performed.push("GitHub REST API artifact metadata inspection".to_string());
    let artifacts_res = provider.get_artifacts(&target.owner, &target.repo, target.run_id);

    let evidence_availability_status = match artifacts_res {
        Err(e) => {
            if target.declared_artifacts.is_empty() {
                warnings.push(format!(
                    "GitHub REST API artifact lookup returned error: {e}"
                ));
                "NOT_PROVIDED"
            } else {
                blocking_reasons.push(format!(
                    "GitHub REST API artifact metadata lookup failed: {e}"
                ));
                "UNREACHABLE"
            }
        }
        Ok(observed_artifacts) => {
            if target.declared_artifacts.is_empty() {
                "NOT_PROVIDED"
            } else {
                let mut all_found = true;
                let mut any_found = false;

                for declared in &target.declared_artifacts {
                    let matched_artifact: Option<&GitHubArtifact> =
                        observed_artifacts.iter().find(|a| {
                            if let Some(id) = declared.artifact_id {
                                a.id == id
                            } else {
                                a.name == declared.artifact_name
                            }
                        });

                    if let Some(art) = matched_artifact {
                        any_found = true;
                        if art.expired {
                            expired_references.push(art.name.clone());
                            blocking_reasons.push(format!(
                                "Artifact '{}' (id={}) is expired in GitHub Actions storage",
                                art.name, art.id
                            ));
                        } else if declared.verify_bytes {
                            if declared
                                .expected_digest
                                .as_deref()
                                .unwrap_or("")
                                .is_empty()
                            {
                                warnings.push(format!(
                                    "Artifact '{}' requested byte verification but \
                                     expected_digest was empty or missing",
                                    art.name
                                ));
                            } else {
                                checks_performed.push(format!(
                                    "Artifact byte digest verification for '{}'",
                                    art.name
                                ));
                                match provider.download_artifact_digest(
                                    &target.owner,
                                    &target.repo,
                                    art.id,
                                    10 * 1024 * 1024,
                                ) {
                                    Ok(computed) => {
                                        if let Some(ref expected) = declared.expected_digest {
                                            if computed.to_lowercase() != expected.to_lowercase() {
                                                blocking_reasons.push(format!(
                                                    "Artifact '{}' digest mismatch: \
                                                     exp '{}', got '{}'",
                                                    art.name, expected, computed
                                                ));
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        blocking_reasons.push(format!(
                                            "Failed to download and verify artifact '{}': {}",
                                            art.name, e
                                        ));
                                    }
                                }
                            }
                        }
                    } else {
                        all_found = false;
                        blocking_reasons.push(format!(
                            "Declared artifact '{}' was not found in provider run artifacts",
                            declared.artifact_name
                        ));
                    }
                }

                if !expired_references.is_empty() {
                    "EXPIRED"
                } else if all_found {
                    "AVAILABLE"
                } else if any_found {
                    "PARTIALLY_AVAILABLE"
                } else {
                    "UNREACHABLE"
                }
            }
        }
    };

    // 7. Derive Final Verdict and Exit Code
    let (verdict, exit_code) = if !blocking_reasons.is_empty() {
        ("ASSESSED_FAIL", 2)
    } else if ci_status == "UNVERIFIED_RUN" {
        ("INSUFFICIENT_EVIDENCE", 3)
    } else {
        warnings.push(
            "CI run independently verified; does not attest live branch HEAD freshness".to_string(),
        );
        warnings
            .push("Independent code review not performed; review status is UNREVIEWED".to_string());
        warnings.push("Independent human owner authorization is pending".to_string());
        ("ASSESSED_LIMITATIONS", 1)
    };

    let assessment = json!({
        "schema_version": "0.1.0-draft",
        "assessment_id": assessment_id,
        "evaluator_auth": {
            "execution_context": "LOCAL_SUBPROCESS_PIPE",
            "is_synthetic_mock": false
        },
        "engine": {
            "name": "Evidence-Checkpoint-Engine",
            "version": env!("CARGO_PKG_VERSION"),
            "canonicalization_standard": "RFC8785"
        },
        "evaluation_timestamp": timestamp,
        "evaluated_target": {
            "repository_id": target.expected_repo_id.clone(),
            "repository_owner": target.owner.clone(),
            "repository_name": target.repo.clone(),
            "target_commit_sha": target.commit_sha.clone(),
            "evaluated_checkpoint_id": target.checkpoint_id.clone()
        },
        "dimensions": {
            "evidence_integrity": {
                "status": evidence_integrity_status,
                "digest_match": digest_match,
                "details": integrity_details
            },
            "ci_execution": {
                "status": ci_status,
                "expected_jobs_count": expected_jobs_count,
                "observed_jobs_count": observed_jobs_count,
                "unresolved_jobs": unresolved_jobs
            },
            "evidence_availability": {
                "status": evidence_availability_status,
                "expired_references": expired_references
            },
            "review_status": {
                "status": "UNREVIEWED",
                "tier": "NONE",
                "scope": "No independent review completed for this checkpoint proposal",
                "reviewer_identity": null
            },
            "authorization": {
                "status": "AUTHORIZATION_PENDING",
                "decision_by": null,
                "authorization_scope": null
            },
            "freshness": {
                "status": "HISTORICAL_BASELINE",
                "live_head_sha": null
            }
        },
        "derived_summary": {
            "verdict": verdict,
            "blocking_reasons": blocking_reasons,
            "warnings": warnings
        },
        "claims_breakdown": {
            "source_declared": {
                "claimed_phase": target.declared_phase.clone(),
                "claimed_producer": target.declared_producer.clone(),
                "claimed_checkpoint_id": target.checkpoint_id.clone()
            },
            "machine_assessed": {
                "sha_verified": sha_verified,
                "repo_id_matched": repo_id_matched,
                "schema_conformant": schema_conformant,
                "digest_verified": digest_verified
            },
            "independently_reviewed": {
                "review_verdict": "NOT_EVALUATED",
                "reviewer": null,
                "audit_scope": null
            },
            "human_authorized": {
                "authorized": false,
                "owner_signature_ref": null
            }
        },
        "checks_performed": checks_performed,
        "checks_omitted": checks_omitted,
        "exit_code": exit_code
    });

    validate_and_wrap(
        &store,
        VerificationOutput {
            exit_code,
            assessment,
        },
    )
}

#[allow(clippy::too_many_arguments)]
fn build_api_failure_assessment(
    store: &SchemaStore,
    target: &TargetEvidenceRequest,
    assessment_id: &str,
    timestamp: &str,
    resource: &str,
    error: &GitHubClientError,
    checks_performed: Vec<String>,
    checks_omitted: Vec<String>,
) -> Result<VerificationOutput, VerifierError> {
    let warning = format!("GitHub API request failed for {}: {}", resource, error);

    let assessment = json!({
        "schema_version": "0.1.0-draft",
        "assessment_id": assessment_id,
        "evaluator_auth": {
            "execution_context": "LOCAL_SUBPROCESS_PIPE",
            "is_synthetic_mock": false
        },
        "engine": {
            "name": "Evidence-Checkpoint-Engine",
            "version": env!("CARGO_PKG_VERSION"),
            "canonicalization_standard": "RFC8785"
        },
        "evaluation_timestamp": timestamp,
        "evaluated_target": {
            "repository_id": target.expected_repo_id.clone(),
            "repository_owner": target.owner.clone(),
            "repository_name": target.repo.clone(),
            "target_commit_sha": target.commit_sha.clone(),
            "evaluated_checkpoint_id": target.checkpoint_id.clone()
        },
        "dimensions": {
            "evidence_integrity": {
                "status": "NOT_EVALUATED",
                "digest_match": false,
                "details": "GitHub API evidence retrieval failed"
            },
            "ci_execution": {
                "status": "UNVERIFIED_RUN",
                "expected_jobs_count": 0,
                "observed_jobs_count": 0,
                "unresolved_jobs": []
            },
            "evidence_availability": {
                "status": "UNREACHABLE",
                "expired_references": []
            },
            "review_status": {
                "status": "NOT_EVALUATED",
                "tier": "NONE",
                "scope": "Evaluation halted due to provider API failure",
                "reviewer_identity": null
            },
            "authorization": {
                "status": "AUTHORIZATION_PENDING",
                "decision_by": null,
                "authorization_scope": null
            },
            "freshness": {
                "status": "NOT_EVALUATED",
                "live_head_sha": null
            }
        },
        "derived_summary": {
            "verdict": "INSUFFICIENT_EVIDENCE",
            "blocking_reasons": vec![warning.clone()],
            "warnings": vec![warning]
        },
        "claims_breakdown": {
            "source_declared": {
                "claimed_phase": target.declared_phase.clone(),
                "claimed_producer": target.declared_producer.clone(),
                "claimed_checkpoint_id": target.checkpoint_id.clone()
            },
            "machine_assessed": {
                "sha_verified": false,
                "repo_id_matched": false,
                "schema_conformant": false,
                "digest_verified": false
            },
            "independently_reviewed": {
                "review_verdict": "NOT_EVALUATED",
                "reviewer": null,
                "audit_scope": null
            },
            "human_authorized": {
                "authorized": false,
                "owner_signature_ref": null
            }
        },
        "checks_performed": checks_performed,
        "checks_omitted": checks_omitted,
        "exit_code": 3
    });

    validate_and_wrap(
        store,
        VerificationOutput {
            exit_code: 3,
            assessment,
        },
    )
}
