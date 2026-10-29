// assessment.rs — Six-dimensional assessment builder and verdict logic.
//
// Fundamental rule: the offline engine MUST NOT assert that GitHub CI,
// independent review, or human authorization has been verified when it has
// no authenticated evidence for those facts.
//
// For the offline MVP, ASSESSED_PROVISIONAL_PASS (exit 0) is never achievable.
// The best possible outcome for a structurally valid envelope with matching
// digest is ASSESSED_LIMITATIONS (exit 1).

use crate::verifier::{error::VerifierError, schema::SchemaStore, semantic::SemanticResult};
use chrono::Utc;
use serde_json::{json, Value};
use std::path::Path;
use uuid::Uuid;

/// Output of the full verification pipeline.
pub struct VerificationOutput {
    pub assessment: Value,
    pub exit_code: i32,
}

/// Helper to validate emitted assessment against assessment-output.schema.json.
/// In debug mode (including tests), panics on schema violation to fail immediately.
/// In release mode, logs error to stderr.
fn validate_emitted_assessment(store: &SchemaStore, output: &VerificationOutput) {
    if let Err(e) = store.validate_assessment(&output.assessment) {
        eprintln!("ECE_ASSESSMENT_SCHEMA_VIOLATION: {e}");
        #[cfg(debug_assertions)]
        panic!("Emitted assessment violates assessment-output.schema.json: {e}");
    }
}

/// Extract an optional string property by JSON pointer.
pub fn extract_opt_str(value: &Value, pointer: &str) -> Option<String> {
    value
        .pointer(pointer)
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned())
}

/// Extract and validate repository_id: must match ^[1-9][0-9]{0,19}$ and parse as u64.
/// Returns None if absent or invalid.
pub fn extract_valid_repo_id(payload: &Value) -> Option<String> {
    let s = payload.pointer("/repository/github_rest_id")?.as_str()?;
    if !s.is_empty()
        && s.chars().all(|c| c.is_ascii_digit())
        && !s.starts_with('0')
        && s.len() <= 20
        && s.parse::<u64>().is_ok()
    {
        Some(s.to_owned())
    } else {
        None
    }
}

/// Extract and validate commit_sha: must match ^[0-9a-f]{40,64}$.
/// Returns None if absent or invalid.
pub fn extract_valid_commit_sha(payload: &Value) -> Option<String> {
    let s = payload.pointer("/source/commit_sha")?.as_str()?;
    if (s.len() == 40 || s.len() == 64)
        && s.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    {
        Some(s.to_owned())
    } else {
        None
    }
}

/// Construct a complete assessment for a valid envelope with matching digest.
///
/// This is the SUCCESS path: schema valid, digest matches, semantic checks passed.
/// Result is always ASSESSED_LIMITATIONS (exit 1) for the offline MVP, because
/// CI execution, independent review, and human authorization cannot be
/// authenticated without live GitHub API access.
pub fn build_assessment_limitations(
    payload: &Value,
    digest: &str,
    semantic: &SemanticResult,
) -> VerificationOutput {
    let assessment_id = format!("eval-{}", Uuid::new_v4());
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let repo_id = extract_valid_repo_id(payload);
    let repo_owner = extract_opt_str(payload, "/repository/owner");
    let repo_name = extract_opt_str(payload, "/repository/name");
    let commit_sha = extract_valid_commit_sha(payload);
    let checkpoint_id = extract_opt_str(payload, "/checkpoint_id");

    let ci_status = if payload.get("ci_evidence").is_some() {
        "UNVERIFIED_RUN"
    } else {
        "NOT_APPLICABLE"
    };

    let expected_jobs = payload
        .pointer("/ci_evidence/expected_jobs")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let observed_jobs = payload
        .pointer("/ci_evidence/observed_jobs")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    let unresolved_jobs: Vec<Value> = semantic.unresolved_jobs.iter().map(|s| json!(s)).collect();

    let mut all_warnings: Vec<Value> = vec![
        json!("OFFLINE_MVP: CI execution authenticity cannot be established without live GitHub API access"),
        json!("OFFLINE_MVP: Independent reviewer identity cannot be verified offline"),
        json!("OFFLINE_MVP: Human owner authorization cannot be established offline"),
        json!(format!("Digest {digest} establishes content consistency only; it does not prove GitHub provenance or reviewer identity")),
    ];
    for w in &semantic.warnings {
        all_warnings.push(json!(w));
    }

    let mut checks_performed = vec![
        json!("UTF-8 validation"),
        json!("Duplicate JSON key detection"),
        json!("JSON nesting depth check (max 32)"),
        json!("JSON Schema Draft 2020-12 validation (checkpoint-envelope.schema.json)"),
        json!("JSON Schema Draft 2020-12 validation (checkpoint-input.schema.json)"),
        json!("RFC 8785 JCS canonicalization of payload"),
        json!("SHA-256 digest computation and envelope_manifest_digest comparison"),
        json!("uint64 string bounds check (github_rest_id, run_id)"),
        json!("git SHA algorithm/length consistency"),
        json!("CI job status/conclusion consistency"),
        json!("Duplicate expected_job name detection"),
        json!("Expected-vs-observed job comparison"),
        json!("Self-injection attempt detection"),
    ];
    if payload.get("ci_evidence").is_some() {
        checks_performed.push(json!("CI policy reference presence check"));
    }

    let checks_omitted = vec![
        json!("Live GitHub REST API authentication and repository owner validation"),
        json!("CI workflow run cryptographic verification against GitHub Actions API"),
        json!("Independent reviewer identity authentication"),
        json!("Human owner authorization cryptographic signature verification"),
        json!("Policy baseline digest comparison against locally pinned trusted policy"),
        json!("Freshness/live HEAD SHA comparison via GitHub API"),
        json!("Artifact download and digest re-verification"),
    ];

    let claimed_phase = extract_opt_str(payload, "/phase");
    let claimed_producer = extract_opt_str(payload, "/producer/declared_by");
    let claimed_checkpoint = extract_opt_str(payload, "/checkpoint_id");

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
        "evaluation_timestamp": now,
        "evaluated_target": {
            "repository_id": repo_id,
            "repository_owner": repo_owner,
            "repository_name": repo_name,
            "target_commit_sha": commit_sha,
            "evaluated_checkpoint_id": checkpoint_id
        },
        "dimensions": {
            "evidence_integrity": {
                "status": "VALID",
                "digest_match": true,
                "details": format!("Envelope digest {digest} verified: payload content is consistent with declared digest. This establishes content integrity only, not provenance.")
            },
            "ci_execution": {
                "status": ci_status,
                "expected_jobs_count": expected_jobs,
                "observed_jobs_count": observed_jobs,
                "unresolved_jobs": unresolved_jobs
            },
            "evidence_availability": {
                "status": "NOT_PROVIDED",
                "expired_references": []
            },
            "review_status": {
                "status": "UNREVIEWED",
                "tier": "NONE",
                "scope": "Offline MVP: independent review verification unavailable",
                "reviewer_identity": null
            },
            "authorization": {
                "status": "AUTHORIZATION_PENDING",
                "decision_by": null,
                "authorization_scope": "Offline MVP: human owner authorization cannot be established"
            },
            "freshness": {
                "status": "UNKNOWN_HEAD",
                "live_head_sha": null
            }
        },
        "derived_summary": {
            "verdict": "ASSESSED_LIMITATIONS",
            "blocking_reasons": [],
            "warnings": all_warnings
        },
        "claims_breakdown": {
            "source_declared": {
                "claimed_phase": claimed_phase,
                "claimed_producer": claimed_producer,
                "claimed_checkpoint_id": claimed_checkpoint
            },
            "machine_assessed": {
                "sha_verified": false,
                "repo_id_matched": false,
                "schema_conformant": true,
                "digest_verified": true
            },
            "independently_reviewed": {
                "review_verdict": "NOT_EVALUATED_OFFLINE"
            },
            "human_authorized": {
                "authorized": false,
                "owner_signature_ref": null
            }
        },
        "checks_performed": checks_performed,
        "checks_omitted": checks_omitted,
        "exit_code": 1
    });

    VerificationOutput {
        assessment,
        exit_code: 1,
    }
}

/// Construct an assessment for a digest mismatch (ASSESSED_FAIL, exit 2).
pub fn build_assessment_digest_fail(
    payload: &Value,
    expected: &str,
    computed: &str,
) -> VerificationOutput {
    let assessment_id = format!("eval-{}", Uuid::new_v4());
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let repo_id = extract_valid_repo_id(payload);
    let repo_owner = extract_opt_str(payload, "/repository/owner");
    let repo_name = extract_opt_str(payload, "/repository/name");
    let commit_sha = extract_valid_commit_sha(payload);
    let checkpoint_id = extract_opt_str(payload, "/checkpoint_id");

    let claimed_phase = extract_opt_str(payload, "/phase");
    let claimed_producer = extract_opt_str(payload, "/producer/declared_by");
    let claimed_checkpoint = extract_opt_str(payload, "/checkpoint_id");

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
        "evaluation_timestamp": now,
        "evaluated_target": {
            "repository_id": repo_id,
            "repository_owner": repo_owner,
            "repository_name": repo_name,
            "target_commit_sha": commit_sha,
            "evaluated_checkpoint_id": checkpoint_id
        },
        "dimensions": {
            "evidence_integrity": {
                "status": "TAMPERED",
                "digest_match": false,
                "details": format!(
                    "Digest mismatch: envelope declares {expected} but computed {computed}. \
                     The payload has been modified after the digest was recorded."
                )
            },
            "ci_execution": {
                "status": "NOT_APPLICABLE",
                "expected_jobs_count": 0,
                "observed_jobs_count": 0,
                "unresolved_jobs": []
            },
            "evidence_availability": {
                "status": "NOT_PROVIDED",
                "expired_references": []
            },
            "review_status": {
                "status": "UNREVIEWED",
                "tier": "NONE",
                "scope": "Verification aborted due to digest mismatch",
                "reviewer_identity": null
            },
            "authorization": {
                "status": "AUTHORIZATION_PENDING",
                "decision_by": null,
                "authorization_scope": null
            },
            "freshness": {
                "status": "UNKNOWN_HEAD",
                "live_head_sha": null
            }
        },
        "derived_summary": {
            "verdict": "ASSESSED_FAIL",
            "blocking_reasons": [
                format!("DIGEST_MISMATCH: expected={expected} computed={computed}")
            ],
            "warnings": []
        },
        "claims_breakdown": {
            "source_declared": {
                "claimed_phase": claimed_phase,
                "claimed_producer": claimed_producer,
                "claimed_checkpoint_id": claimed_checkpoint
            },
            "machine_assessed": {
                "sha_verified": false,
                "repo_id_matched": false,
                "schema_conformant": true,
                "digest_verified": false
            },
            "independently_reviewed": {
                "review_verdict": "NOT_EVALUATED"
            },
            "human_authorized": {
                "authorized": false,
                "owner_signature_ref": null
            }
        },
        "checks_performed": [
            "UTF-8 validation",
            "Duplicate JSON key detection",
            "JSON nesting depth check",
            "JSON Schema Draft 2020-12 validation (envelope)",
            "RFC 8785 JCS canonicalization of payload",
            "SHA-256 digest computation"
        ],
        "checks_omitted": [
            "Semantic validation (aborted after digest mismatch)",
            "CI execution verification",
            "Independent review verification",
            "Human authorization verification"
        ],
        "exit_code": 2
    });

    VerificationOutput {
        assessment,
        exit_code: 2,
    }
}

/// Construct an error/refused assessment (exit 3 or 4).
/// Never synthesizes fake repository IDs or commit SHAs.
pub fn build_assessment_refused(
    error: &VerifierError,
    payload: Option<&Value>,
) -> VerificationOutput {
    let assessment_id = format!("eval-{}", Uuid::new_v4());
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let exit_code = error.exit_code();
    let verdict = "REFUSED";

    let repo_id = payload.and_then(extract_valid_repo_id);
    let repo_owner = payload.and_then(|p| extract_opt_str(p, "/repository/owner"));
    let repo_name = payload.and_then(|p| extract_opt_str(p, "/repository/name"));
    let commit_sha = payload.and_then(extract_valid_commit_sha);
    let checkpoint_id = payload.and_then(|p| extract_opt_str(p, "/checkpoint_id"));

    let claimed_phase = payload.and_then(|p| extract_opt_str(p, "/phase"));
    let claimed_producer = payload.and_then(|p| extract_opt_str(p, "/producer/declared_by"));
    let claimed_checkpoint = payload.and_then(|p| extract_opt_str(p, "/checkpoint_id"));

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
        "evaluation_timestamp": now,
        "evaluated_target": {
            "repository_id": repo_id,
            "repository_owner": repo_owner,
            "repository_name": repo_name,
            "target_commit_sha": commit_sha,
            "evaluated_checkpoint_id": checkpoint_id
        },
        "dimensions": {
            "evidence_integrity": {
                "status": "CORRUPT",
                "digest_match": false,
                "details": format!("[{}] {}", error.class(), error)
            },
            "ci_execution": {
                "status": "NOT_APPLICABLE",
                "expected_jobs_count": 0,
                "observed_jobs_count": 0,
                "unresolved_jobs": []
            },
            "evidence_availability": {
                "status": "NOT_PROVIDED",
                "expired_references": []
            },
            "review_status": {
                "status": "UNREVIEWED",
                "tier": "NONE",
                "scope": "Verification refused due to input or operational error",
                "reviewer_identity": null
            },
            "authorization": {
                "status": "AUTHORIZATION_PENDING",
                "decision_by": null,
                "authorization_scope": null
            },
            "freshness": {
                "status": "UNKNOWN_HEAD",
                "live_head_sha": null
            }
        },
        "derived_summary": {
            "verdict": verdict,
            "blocking_reasons": [
                format!("[{}] {}", error.class(), error)
            ],
            "warnings": []
        },
        "claims_breakdown": {
            "source_declared": {
                "claimed_phase": claimed_phase,
                "claimed_producer": claimed_producer,
                "claimed_checkpoint_id": claimed_checkpoint
            },
            "machine_assessed": {
                "sha_verified": false,
                "repo_id_matched": false,
                "schema_conformant": false
            },
            "independently_reviewed": {
                "review_verdict": "NOT_EVALUATED"
            },
            "human_authorized": {
                "authorized": false,
                "owner_signature_ref": null
            }
        },
        "checks_performed": [],
        "checks_omitted": [
            "Full verification (aborted due to fatal input or operational error)"
        ],
        "exit_code": exit_code
    });

    VerificationOutput {
        assessment,
        exit_code,
    }
}

/// Construct a diagnostic-only assessment for `inspect-untrusted-payload`.
///
/// Always returns exit 3. Emits prominent UNTRUSTED warnings.
/// Never claims trusted provenance or acceptance.
pub fn build_assessment_inspect_untrusted(
    payload: &Value,
    schema_errors: Option<&str>,
    semantic_result: Option<&SemanticResult>,
) -> VerificationOutput {
    let assessment_id = format!("eval-{}", Uuid::new_v4());
    let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let repo_id = extract_valid_repo_id(payload);
    let repo_owner = extract_opt_str(payload, "/repository/owner");
    let repo_name = extract_opt_str(payload, "/repository/name");
    let commit_sha = extract_valid_commit_sha(payload);
    let checkpoint_id = extract_opt_str(payload, "/checkpoint_id");

    let claimed_phase = extract_opt_str(payload, "/phase");
    let claimed_producer = extract_opt_str(payload, "/producer/declared_by");
    let claimed_checkpoint = extract_opt_str(payload, "/checkpoint_id");

    let schema_status = if schema_errors.is_some() {
        "CORRUPT"
    } else {
        "VALID"
    };
    let schema_detail = schema_errors
        .map(|e| format!("Schema errors: {e}"))
        .unwrap_or_else(|| {
            "Structural schema validation passed (envelope digest NOT verified)".to_owned()
        });

    let mut warnings = vec![
        json!("UNTRUSTED_DIAGNOSTIC: This is inspect-untrusted-payload output — envelope digest verification was bypassed"),
        json!("UNTRUSTED_DIAGNOSTIC: This output does NOT constitute a verification result"),
        json!("UNTRUSTED_DIAGNOSTIC: Use verify-envelope for full verification including digest check"),
    ];
    if let Some(sem) = semantic_result {
        for w in &sem.warnings {
            warnings.push(json!(w));
        }
    }

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
        "evaluation_timestamp": now,
        "evaluated_target": {
            "repository_id": repo_id,
            "repository_owner": repo_owner,
            "repository_name": repo_name,
            "target_commit_sha": commit_sha,
            "evaluated_checkpoint_id": checkpoint_id
        },
        "dimensions": {
            "evidence_integrity": {
                "status": schema_status,
                "digest_match": false,
                "details": schema_detail
            },
            "ci_execution": {
                "status": "NOT_APPLICABLE",
                "expected_jobs_count": 0,
                "observed_jobs_count": 0,
                "unresolved_jobs": []
            },
            "evidence_availability": {
                "status": "NOT_PROVIDED",
                "expired_references": []
            },
            "review_status": {
                "status": "UNREVIEWED",
                "tier": "NONE",
                "scope": "Diagnostic inspection only — not a verification result",
                "reviewer_identity": null
            },
            "authorization": {
                "status": "AUTHORIZATION_PENDING",
                "decision_by": null,
                "authorization_scope": null
            },
            "freshness": {
                "status": "UNKNOWN_HEAD",
                "live_head_sha": null
            }
        },
        "derived_summary": {
            "verdict": "REFUSED",
            "blocking_reasons": [
                "INSPECT_UNTRUSTED: envelope digest verification bypassed — result is diagnostic only"
            ],
            "warnings": warnings
        },
        "claims_breakdown": {
            "source_declared": {
                "claimed_phase": claimed_phase,
                "claimed_producer": claimed_producer,
                "claimed_checkpoint_id": claimed_checkpoint
            },
            "machine_assessed": {
                "sha_verified": false,
                "repo_id_matched": false,
                "schema_conformant": schema_errors.is_none()
            },
            "independently_reviewed": {
                "review_verdict": "NOT_EVALUATED"
            },
            "human_authorized": {
                "authorized": false,
                "owner_signature_ref": null
            }
        },
        "checks_performed": [
            "UTF-8 validation",
            "Duplicate JSON key detection",
            "JSON nesting depth check",
            "JSON Schema Draft 2020-12 structural validation (payload only)"
        ],
        "checks_omitted": [
            "Envelope digest verification (bypassed — inspect-untrusted-payload mode)",
            "RFC 8785 JCS canonicalization",
            "SHA-256 digest computation",
            "CI execution verification",
            "Independent review verification",
            "Human authorization verification"
        ],
        "exit_code": 3
    });

    VerificationOutput {
        assessment,
        exit_code: 3,
    }
}

/// Run the full `verify-envelope` pipeline.
///
/// Order of operations:
/// 1. Bounded parse (size, UTF-8, duplicate keys, depth)
/// 2. Schema validation (envelope schema → validates payload via $ref)
/// 3. Digest verification (RFC 8785 + SHA-256)
/// 4. Semantic checks on the inner payload
/// 5. Build six-dimensional assessment
/// 6. Validate assessment against assessment-output.schema.json
pub fn verify_envelope(input_bytes: &[u8], schema_dir: &Path) -> VerificationOutput {
    // Load schemas (may fail if schema dir missing — operational error, exit 4)
    let store = match SchemaStore::load(schema_dir) {
        Ok(s) => s,
        Err(e) => return build_assessment_refused(&e, None),
    };

    // 1. Bounded parse
    let envelope = match crate::verifier::parser::parse_bounded_json(input_bytes) {
        Ok(v) => v,
        Err(e) => {
            let out = build_assessment_refused(&e, None);
            validate_emitted_assessment(&store, &out);
            return out;
        }
    };

    // 2. Schema validation (envelope validates payload via $ref inline)
    if let Err(e) = store.validate_envelope(&envelope) {
        let payload = envelope.get("payload");
        let out = build_assessment_refused(&e, payload);
        validate_emitted_assessment(&store, &out);
        return out;
    }

    // Extract payload for downstream steps
    let payload = match envelope.get("payload") {
        Some(p) => p.clone(),
        None => {
            let out = build_assessment_refused(
                &VerifierError::InternalError(
                    "envelope missing payload after schema validation".to_owned(),
                ),
                None,
            );
            validate_emitted_assessment(&store, &out);
            return out;
        }
    };

    // 3. Digest verification
    let digest = match crate::verifier::digest::verify_envelope_digest(&envelope) {
        Ok(d) => d,
        Err(VerifierError::DigestMismatch { expected, computed }) => {
            let out = build_assessment_digest_fail(&payload, &expected, &computed);
            validate_emitted_assessment(&store, &out);
            return out;
        }
        Err(e) => {
            let out = build_assessment_refused(&e, Some(&payload));
            validate_emitted_assessment(&store, &out);
            return out;
        }
    };

    // 4. Semantic checks
    let semantic = match crate::verifier::semantic::run_semantic_checks(&payload) {
        Ok(s) => s,
        Err(e) => {
            let out = build_assessment_refused(&e, Some(&payload));
            validate_emitted_assessment(&store, &out);
            return out;
        }
    };

    // 5. Build limitations assessment (best achievable offline)
    let out = build_assessment_limitations(&payload, &digest, &semantic);
    validate_emitted_assessment(&store, &out);
    out
}

/// Run the `inspect-untrusted-payload` pipeline (diagnostic only).
///
/// Always exits 3. Emits prominent UNTRUSTED warnings.
pub fn inspect_payload(input_bytes: &[u8], schema_dir: &Path) -> VerificationOutput {
    // Load schemas
    let store = match SchemaStore::load(schema_dir) {
        Ok(s) => s,
        Err(e) => return build_assessment_refused(&e, None),
    };

    // Bounded parse
    let payload = match crate::verifier::parser::parse_bounded_json(input_bytes) {
        Ok(v) => v,
        Err(e) => {
            let out = build_assessment_refused(&e, None);
            validate_emitted_assessment(&store, &out);
            return out;
        }
    };

    // Schema validation (payload only — no envelope digest)
    let schema_error = store
        .validate_payload(&payload)
        .err()
        .map(|e| e.to_string());

    // Semantic checks (best-effort, non-fatal for this mode)
    let semantic = crate::verifier::semantic::run_semantic_checks(&payload).ok();

    let out =
        build_assessment_inspect_untrusted(&payload, schema_error.as_deref(), semantic.as_ref());
    validate_emitted_assessment(&store, &out);
    out
}
