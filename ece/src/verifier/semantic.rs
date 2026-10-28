// semantic.rs — Semantic validation rules for evidence checkpoint payloads.
//
// These checks operate on the parsed, schema-valid payload and enforce:
//   - uint64 string parsing with strict overflow detection
//   - git SHA algorithm/length consistency
//   - Duplicate expected job names
//   - Job status/conclusion consistency
//   - Self-declared acceptance injection detection
//   - Missing expected jobs (produces warnings, not fatal errors)
//
// All functions are pure (no I/O) and deterministic.

use crate::verifier::error::{ErrorCode, VerifierError};
use serde_json::Value;
use std::collections::HashSet;

/// Result of semantic checks: a list of non-fatal warning strings.
/// Fatal violations return Err(VerifierError::SemanticError).
#[derive(Debug, Clone)]
pub struct SemanticResult {
    pub warnings: Vec<String>,
    pub unresolved_jobs: Vec<String>,
}

/// Run all semantic checks on a schema-valid payload.
///
/// Returns `Ok(SemanticResult)` for warnings, `Err` for fatal violations.
pub fn run_semantic_checks(payload: &Value) -> Result<SemanticResult, VerifierError> {
    let mut warnings = Vec::new();
    let mut unresolved_jobs = Vec::new();

    // 1. Schema version (belt-and-suspenders after schema validation)
    check_schema_version(payload)?;

    // 2. Repository ID: uint64 bounds check
    if let Some(repo_id) = payload.pointer("/repository/github_rest_id") {
        let s = repo_id.as_str().unwrap_or("");
        check_uint64_string(s, "repository.github_rest_id")?;
    }

    // 3. Source commit SHA / algorithm consistency
    check_git_sha_consistency(payload)?;

    // 4. CI evidence semantic checks
    if let Some(ci) = payload.get("ci_evidence") {
        // 4a. run_id uint64 bounds
        if let Some(run_id) = ci.get("run_id").and_then(|v| v.as_str()) {
            check_uint64_string(run_id, "ci_evidence.run_id")?;
        }

        // 4b. Duplicate expected job names
        check_duplicate_jobs(ci)?;

        // 4c. Job status/conclusion consistency
        check_job_status_conclusion(ci)?;

        // 4d. Missing expected jobs
        let missing = check_missing_jobs(ci);
        if !missing.is_empty() {
            let msg = format!(
                "Expected jobs not observed: {}",
                missing.join(", ")
            );
            warnings.push(msg);
            unresolved_jobs.extend(missing);
        }

        // 4e. Missing policy reference (schema enforces this too, but belt-and-suspenders)
        if ci.get("expected_policy_ref").is_none() {
            return Err(VerifierError::SemanticError {
                code: ErrorCode::MissingPolicyReference,
                detail: "ci_evidence present but expected_policy_ref is absent".to_owned(),
            });
        }

        // 4f. Policy digest is present but not verified against any local baseline
        //     (offline MVP cannot verify policy authenticity — emit warning)
        if ci.get("expected_policy_digest").is_some() {
            warnings.push(
                "ci_evidence.expected_policy_digest cannot be authenticated offline: \
                 UNVERIFIED_POLICY — policy digest is producer-supplied and not matched \
                 against a locally pinned policy baseline"
                    .to_owned(),
            );
        }
    }

    // 5. Self-injection guard on review_declarations
    check_review_self_injection(payload, &mut warnings)?;

    Ok(SemanticResult {
        warnings,
        unresolved_jobs,
    })
}

/// Check schema_version is exactly "0.1.0-draft".
fn check_schema_version(payload: &Value) -> Result<(), VerifierError> {
    let v = payload
        .get("schema_version")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if v != "0.1.0-draft" {
        return Err(VerifierError::SemanticError {
            code: ErrorCode::UnsupportedVersion,
            detail: format!("schema_version {v:?} is not supported; only \"0.1.0-draft\" is accepted"),
        });
    }
    Ok(())
}

/// Parse a decimal string as u64, rejecting overflow and leading zeros.
///
/// The JSON Schema regex `^[1-9][0-9]{0,19}$` permits 20-digit strings like
/// "18446744073709551616" (2^64) which exceeds u64::MAX. This function catches them.
fn check_uint64_string(s: &str, field: &str) -> Result<u64, VerifierError> {
    // Leading-zero guard (schema should catch these, but be explicit)
    if s.starts_with('0') {
        return Err(VerifierError::SemanticError {
            code: ErrorCode::LeadingZeroIdentifier,
            detail: format!("{field}: value {s:?} has a leading zero"),
        });
    }
    // Length guard: u64::MAX has 20 digits; anything longer is definitely overflow
    if s.len() > 20 {
        return Err(VerifierError::SemanticError {
            code: ErrorCode::Uint64Overflow,
            detail: format!("{field}: value {s:?} exceeds uint64::MAX"),
        });
    }
    s.parse::<u64>().map_err(|_| VerifierError::SemanticError {
        code: ErrorCode::Uint64Overflow,
        detail: format!("{field}: value {s:?} exceeds uint64::MAX (18446744073709551615)"),
    })
}

/// Verify that `source.commit_sha` length matches `source.git_hash_algorithm`.
fn check_git_sha_consistency(payload: &Value) -> Result<(), VerifierError> {
    let source = match payload.get("source") {
        Some(s) => s,
        None => return Ok(()),
    };
    let algo = source
        .get("git_hash_algorithm")
        .and_then(|v| v.as_str())
        .unwrap_or("sha1");
    let commit_sha = source
        .get("commit_sha")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let tree_hash = source
        .get("tree_hash")
        .and_then(|v| v.as_str());

    let expected_len = match algo {
        "sha1" => 40usize,
        "sha256" => 64usize,
        other => {
            return Err(VerifierError::SemanticError {
                code: ErrorCode::GitAlgorithmLengthMismatch,
                detail: format!("unknown git_hash_algorithm: {other:?}"),
            });
        }
    };

    if !commit_sha.is_empty() && commit_sha.len() != expected_len {
        return Err(VerifierError::SemanticError {
            code: ErrorCode::GitAlgorithmLengthMismatch,
            detail: format!(
                "commit_sha length {} does not match git_hash_algorithm {algo:?} \
                 (expected {expected_len} chars)",
                commit_sha.len()
            ),
        });
    }

    if let Some(th) = tree_hash {
        if th.len() != expected_len {
            return Err(VerifierError::SemanticError {
                code: ErrorCode::GitAlgorithmLengthMismatch,
                detail: format!(
                    "tree_hash length {} does not match git_hash_algorithm {algo:?} \
                     (expected {expected_len} chars)",
                    th.len()
                ),
            });
        }
    }

    Ok(())
}

/// Check for duplicate names in `ci_evidence.expected_jobs`.
fn check_duplicate_jobs(ci: &Value) -> Result<(), VerifierError> {
    let expected = match ci.get("expected_jobs").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return Ok(()),
    };
    let mut seen = HashSet::new();
    for job in expected {
        let name = job.as_str().unwrap_or("");
        if !seen.insert(name) {
            return Err(VerifierError::SemanticError {
                code: ErrorCode::DuplicateJobName,
                detail: format!("duplicate expected_job name: {name:?}"),
            });
        }
    }
    Ok(())
}

/// Check job status/conclusion consistency in `ci_evidence.observed_jobs`.
///
/// Rules (per GOV-00 schema F-01 fix):
/// - status == "completed" → conclusion MUST be present and be a non-null string
/// - status == "queued" or "in_progress" → conclusion MUST NOT be present (or must be null/absent)
fn check_job_status_conclusion(ci: &Value) -> Result<(), VerifierError> {
    let observed = match ci.get("observed_jobs").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return Ok(()),
    };

    for job in observed {
        let name = job.get("name").and_then(|v| v.as_str()).unwrap_or("<unnamed>");
        let status = job.get("status").and_then(|v| v.as_str()).unwrap_or("");
        let conclusion = job.get("conclusion");

        match status {
            "completed" => {
                // conclusion must be present and non-null
                match conclusion {
                    None | Some(Value::Null) => {
                        return Err(VerifierError::SemanticError {
                            code: ErrorCode::InvalidJobStatusConclusion,
                            detail: format!(
                                "job {name:?}: status is \"completed\" but conclusion is absent or null"
                            ),
                        });
                    }
                    Some(Value::String(c)) if !c.is_empty() => {} // valid
                    Some(other) => {
                        return Err(VerifierError::SemanticError {
                            code: ErrorCode::InvalidJobStatusConclusion,
                            detail: format!(
                                "job {name:?}: conclusion has unexpected type: {other:?}"
                            ),
                        });
                    }
                }
            }
            "queued" | "in_progress" => {
                // conclusion must be absent or null
                match conclusion {
                    None | Some(Value::Null) => {} // valid
                    Some(c) => {
                        return Err(VerifierError::SemanticError {
                            code: ErrorCode::InvalidJobStatusConclusion,
                            detail: format!(
                                "job {name:?}: status is {status:?} but conclusion is \
                                 set to {c:?} (must be absent or null)"
                            ),
                        });
                    }
                }
            }
            other => {
                // Unknown status values are an unexpected condition
                return Err(VerifierError::SemanticError {
                    code: ErrorCode::InvalidJobStatusConclusion,
                    detail: format!("job {name:?}: unknown status {other:?}"),
                });
            }
        }
    }
    Ok(())
}

/// Return a list of expected job names that are not present in observed_jobs.
fn check_missing_jobs(ci: &Value) -> Vec<String> {
    let expected = match ci.get("expected_jobs").and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return Vec::new(),
    };
    let observed: HashSet<&str> = ci
        .get("observed_jobs")
        .and_then(|v| v.as_array())
        .map(|jobs| {
            jobs.iter()
                .filter_map(|j| j.get("name").and_then(|n| n.as_str()))
                .collect()
        })
        .unwrap_or_default();

    expected
        .iter()
        .filter_map(|j| j.as_str())
        .filter(|name| !observed.contains(name))
        .map(|s| s.to_owned())
        .collect()
}

/// Detect self-injection in review_declarations: an AI agent claiming independent review.
///
/// This is a soft guard; the schema also enforces review_type values.
fn check_review_self_injection(payload: &Value, warnings: &mut Vec<String>) -> Result<(), VerifierError> {
    let reviews = match payload
        .get("review_declarations")
        .and_then(|v| v.as_array())
    {
        Some(a) => a,
        None => return Ok(()),
    };

    let producer_role = payload
        .pointer("/producer/role")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    for review in reviews {
        let review_type = review
            .get("review_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let reviewer_id = review
            .get("reviewer_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // An AI agent claiming human_independent review is a self-injection attempt
        if producer_role == "agent" && review_type == "human_independent" {
            return Err(VerifierError::SemanticError {
                code: ErrorCode::SelfInjectionAttempt,
                detail: format!(
                    "producer.role is \"agent\" but review_declarations contains \
                     review_type \"human_independent\" for reviewer {reviewer_id:?}; \
                     this is a self-injection attempt"
                ),
            });
        }

        // An AI agent claiming its own work as a trusted authority
        if review_type == "ai_agent_self_report" {
            warnings.push(format!(
                "review_declarations contains ai_agent_self_report from {reviewer_id:?}: \
                 this is a declared claim, not independently verified"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn load_payload(fixture_name: &str) -> Value {
        let path = format!(
            "{}/../docs/gov-00/fixtures/{fixture_name}",
            env!("CARGO_MANIFEST_DIR")
        );
        let bytes = std::fs::read(&path).unwrap_or_else(|_| panic!("cannot read {path}"));
        serde_json::from_slice(&bytes).expect("valid JSON")
    }

    #[test]
    fn valid_bootstrap_passes_semantic() {
        let payload = load_payload("input_valid_bootstrap_checkpoint.json");
        let result = run_semantic_checks(&payload);
        assert!(result.is_ok(), "bootstrap should pass semantic: {result:?}");
    }

    #[test]
    fn uint64_overflow_edge_rejected() {
        let payload = load_payload("input_invalid_uint64_overflow_edge.json");
        // github_rest_id = "18446744073709551616" (2^64, 1 over u64::MAX)
        match run_semantic_checks(&payload) {
            Err(VerifierError::SemanticError { code: ErrorCode::Uint64Overflow, .. }) => {}
            other => panic!("expected Uint64Overflow, got: {other:?}"),
        }
    }

    #[test]
    fn uint64_twenty_nines_rejected() {
        let payload = load_payload("input_invalid_uint64_twenty_nines.json");
        // github_rest_id = "99999999999999999999" (20 nines, > u64::MAX)
        match run_semantic_checks(&payload) {
            Err(VerifierError::SemanticError { code: ErrorCode::Uint64Overflow, .. }) => {}
            other => panic!("expected Uint64Overflow, got: {other:?}"),
        }
    }

    #[test]
    fn sha1_with_sha256_length_rejected() {
        let payload = load_payload("input_invalid_git_sha1_with_sha256_length.json");
        match run_semantic_checks(&payload) {
            Err(VerifierError::SemanticError {
                code: ErrorCode::GitAlgorithmLengthMismatch, ..
            }) => {}
            other => panic!("expected GitAlgorithmLengthMismatch, got: {other:?}"),
        }
    }

    #[test]
    fn sha256_with_sha1_length_rejected() {
        let payload = load_payload("input_invalid_git_sha256_with_sha1_length.json");
        match run_semantic_checks(&payload) {
            Err(VerifierError::SemanticError {
                code: ErrorCode::GitAlgorithmLengthMismatch, ..
            }) => {}
            other => panic!("expected GitAlgorithmLengthMismatch, got: {other:?}"),
        }
    }

    #[test]
    fn unsupported_schema_version_rejected() {
        let payload = load_payload("input_invalid_unsupported_schema_version.json");
        match run_semantic_checks(&payload) {
            Err(VerifierError::SemanticError { code: ErrorCode::UnsupportedVersion, .. }) => {}
            other => panic!("expected UnsupportedVersion, got: {other:?}"),
        }
    }

    #[test]
    fn duplicate_expected_job_names_rejected() {
        let payload = json!({
            "schema_version": "0.1.0-draft",
            "checkpoint_id": "test",
            "phase": "TEST",
            "timestamp": "2026-10-25T00:00:00Z",
            "repository": {"owner": "A", "name": "B", "github_rest_id": "1"},
            "source": {"commit_sha": "bead6837e1ec03c8fe1ac3c8d09fd374fb8b0e02", "git_hash_algorithm": "sha1", "branch": "main"},
            "producer": {"declared_by": "Test", "role": "developer"},
            "integrity": {"canonicalization_algorithm": "RFC8785", "digest_algorithm": "sha256"},
            "ci_evidence": {
                "workflow_path": ".github/workflows/ci.yml",
                "run_id": "100",
                "run_attempt": 1,
                "expected_policy_ref": "policy.json",
                "expected_policy_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "expected_jobs": ["test", "test"],
                "observed_jobs": [{"name": "test", "status": "completed", "conclusion": "success"}]
            }
        });
        match run_semantic_checks(&payload) {
            Err(VerifierError::SemanticError { code: ErrorCode::DuplicateJobName, .. }) => {}
            other => panic!("expected DuplicateJobName, got: {other:?}"),
        }
    }

    #[test]
    fn completed_job_without_conclusion_rejected() {
        let payload = json!({
            "schema_version": "0.1.0-draft",
            "checkpoint_id": "test",
            "phase": "TEST",
            "timestamp": "2026-10-25T00:00:00Z",
            "repository": {"owner": "A", "name": "B", "github_rest_id": "1"},
            "source": {"commit_sha": "bead6837e1ec03c8fe1ac3c8d09fd374fb8b0e02", "git_hash_algorithm": "sha1", "branch": "main"},
            "producer": {"declared_by": "Test", "role": "developer"},
            "integrity": {"canonicalization_algorithm": "RFC8785", "digest_algorithm": "sha256"},
            "ci_evidence": {
                "workflow_path": ".github/workflows/ci.yml",
                "run_id": "100",
                "run_attempt": 1,
                "expected_policy_ref": "policy.json",
                "expected_policy_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "expected_jobs": ["test"],
                "observed_jobs": [{"name": "test", "status": "completed"}]
            }
        });
        match run_semantic_checks(&payload) {
            Err(VerifierError::SemanticError {
                code: ErrorCode::InvalidJobStatusConclusion, ..
            }) => {}
            other => panic!("expected InvalidJobStatusConclusion, got: {other:?}"),
        }
    }

    #[test]
    fn queued_job_with_conclusion_rejected() {
        let payload = json!({
            "schema_version": "0.1.0-draft",
            "checkpoint_id": "test",
            "phase": "TEST",
            "timestamp": "2026-10-25T00:00:00Z",
            "repository": {"owner": "A", "name": "B", "github_rest_id": "1"},
            "source": {"commit_sha": "bead6837e1ec03c8fe1ac3c8d09fd374fb8b0e02", "git_hash_algorithm": "sha1", "branch": "main"},
            "producer": {"declared_by": "Test", "role": "developer"},
            "integrity": {"canonicalization_algorithm": "RFC8785", "digest_algorithm": "sha256"},
            "ci_evidence": {
                "workflow_path": ".github/workflows/ci.yml",
                "run_id": "100",
                "run_attempt": 1,
                "expected_policy_ref": "policy.json",
                "expected_policy_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "expected_jobs": ["test"],
                "observed_jobs": [{"name": "test", "status": "queued", "conclusion": "success"}]
            }
        });
        match run_semantic_checks(&payload) {
            Err(VerifierError::SemanticError {
                code: ErrorCode::InvalidJobStatusConclusion, ..
            }) => {}
            other => panic!("expected InvalidJobStatusConclusion, got: {other:?}"),
        }
    }

    #[test]
    fn missing_expected_jobs_returns_warning() {
        let payload = load_payload("input_invalid_missing_ci_jobs.json");
        // expected: ["test", "lint", "security-audit"], observed: ["test"] only
        let result = run_semantic_checks(&payload).expect("should not be fatal");
        assert!(!result.unresolved_jobs.is_empty(), "should have unresolved jobs");
        assert!(
            result.unresolved_jobs.contains(&"lint".to_owned()),
            "lint should be unresolved"
        );
        assert!(
            result.unresolved_jobs.contains(&"security-audit".to_owned()),
            "security-audit should be unresolved"
        );
    }

    #[test]
    fn missing_policy_ref_rejected() {
        let payload = load_payload("input_invalid_ci_missing_policy.json");
        match run_semantic_checks(&payload) {
            Err(VerifierError::SemanticError {
                code: ErrorCode::MissingPolicyReference, ..
            }) => {}
            other => panic!("expected MissingPolicyReference, got: {other:?}"),
        }
    }

    #[test]
    fn agent_claiming_human_review_rejected() {
        let payload = json!({
            "schema_version": "0.1.0-draft",
            "checkpoint_id": "test",
            "phase": "TEST",
            "timestamp": "2026-10-25T00:00:00Z",
            "repository": {"owner": "A", "name": "B", "github_rest_id": "1"},
            "source": {"commit_sha": "bead6837e1ec03c8fe1ac3c8d09fd374fb8b0e02", "git_hash_algorithm": "sha1", "branch": "main"},
            "producer": {"declared_by": "SomeAgent", "role": "agent"},
            "integrity": {"canonicalization_algorithm": "RFC8785", "digest_algorithm": "sha256"},
            "review_declarations": [
                {"reviewer_id": "agent-001", "review_type": "human_independent",
                 "scope": "code", "target_commit_sha": "bead6837e1ec03c8fe1ac3c8d09fd374fb8b0e02"}
            ]
        });
        match run_semantic_checks(&payload) {
            Err(VerifierError::SemanticError {
                code: ErrorCode::SelfInjectionAttempt, ..
            }) => {}
            other => panic!("expected SelfInjectionAttempt, got: {other:?}"),
        }
    }
}

