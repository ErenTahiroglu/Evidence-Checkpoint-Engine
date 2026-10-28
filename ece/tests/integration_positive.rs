// integration_positive.rs — Positive integration tests for Evidence Checkpoint Engine.

use ece_lib::verifier::{
    canonicalize, compute_digest, inspect_payload, verify_envelope, SchemaStore,
};
use serde_json::{json, Value};
use std::path::PathBuf;

fn schema_dir() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir.push("docs/gov-00/schemas");
    dir
}

fn fixture_dir() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    dir.pop();
    dir.push("docs/gov-00/fixtures");
    dir
}

fn load_fixture(filename: &str) -> Vec<u8> {
    let path = fixture_dir().join(filename);
    std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()))
}

#[test]
fn test_valid_bootstrap_payload_structural_acceptance() {
    let store = SchemaStore::load(&schema_dir()).expect("schemas should load");
    let bytes = load_fixture("input_valid_bootstrap_checkpoint.json");
    let payload: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(store.validate_payload(&payload).is_ok());
}

#[test]
fn test_valid_bootstrap_envelope_content_consistency() {
    let bytes = load_fixture("envelope_valid_bootstrap_checkpoint.json");
    let output = verify_envelope(&bytes, &schema_dir());

    assert_eq!(output.exit_code, 1);
    assert_eq!(
        output.assessment.pointer("/derived_summary/verdict").unwrap(),
        "ASSESSED_LIMITATIONS"
    );
    assert_eq!(
        output
            .assessment
            .pointer("/dimensions/evidence_integrity/status")
            .unwrap(),
        "VALID"
    );
    assert_eq!(
        output
            .assessment
            .pointer("/dimensions/evidence_integrity/digest_match")
            .unwrap(),
        true
    );
    assert_eq!(
        output
            .assessment
            .pointer("/evaluator_auth/is_synthetic_mock")
            .unwrap(),
        false
    );
}

#[test]
fn test_jcs_canonicalization_and_digest_deterministic() {
    let bytes = load_fixture("input_valid_bootstrap_checkpoint.json");
    let payload: Value = serde_json::from_slice(&bytes).unwrap();

    let canonical = canonicalize(&payload).expect("canonicalization must succeed");
    // Verified 653 UTF-8 bytes
    assert_eq!(canonical.len(), 653);

    let digest = compute_digest(&canonical);
    assert_eq!(
        digest,
        "sha256:157aaebc73d8daecfc07aa8e8b706fdcd1d4b5909d55442144a194734c20199d"
    );
}

#[test]
fn test_sha1_and_sha256_identifiers_checked_correctly() {
    let store = SchemaStore::load(&schema_dir()).expect("schemas should load");

    // SHA-1 (40 hex)
    let payload_sha1 = json!({
        "schema_version": "0.1.0-draft",
        "checkpoint_id": "sha1-valid",
        "phase": "TEST-01",
        "timestamp": "2026-10-25T03:37:58Z",
        "repository": {
            "owner": "ErenTahiroglu",
            "name": "Evidence-Checkpoint-Engine",
            "github_rest_id": "1432704795"
        },
        "source": {
            "commit_sha": "bead6837e1ec03c8fe1ac3c8d09fd374fb8b0e02",
            "git_hash_algorithm": "sha1",
            "branch": "development"
        },
        "producer": {
            "declared_by": "Developer",
            "role": "developer"
        },
        "integrity": {
            "canonicalization_algorithm": "RFC8785",
            "digest_algorithm": "sha256"
        }
    });
    assert!(store.validate_payload(&payload_sha1).is_ok());

    // SHA-256 (64 hex)
    let payload_sha256 = json!({
        "schema_version": "0.1.0-draft",
        "checkpoint_id": "sha256-valid",
        "phase": "TEST-01",
        "timestamp": "2026-10-25T03:37:58Z",
        "repository": {
            "owner": "ErenTahiroglu",
            "name": "Evidence-Checkpoint-Engine",
            "github_rest_id": "1432704795"
        },
        "source": {
            "commit_sha": "bead6837e1ec03c8fe1ac3c8d09fd374fb8b0e02000000000000000000000000",
            "git_hash_algorithm": "sha256",
            "branch": "development"
        },
        "producer": {
            "declared_by": "Developer",
            "role": "developer"
        },
        "integrity": {
            "canonicalization_algorithm": "RFC8785",
            "digest_algorithm": "sha256"
        }
    });
    assert!(store.validate_payload(&payload_sha256).is_ok());
}

#[test]
fn test_valid_completed_and_queued_jobs_schema() {
    let store = SchemaStore::load(&schema_dir()).expect("schemas should load");

    let payload = json!({
        "schema_version": "0.1.0-draft",
        "checkpoint_id": "jobs-valid",
        "phase": "TEST-01",
        "timestamp": "2026-10-25T03:37:58Z",
        "repository": {
            "owner": "ErenTahiroglu",
            "name": "Evidence-Checkpoint-Engine",
            "github_rest_id": "1432704795"
        },
        "source": {
            "commit_sha": "bead6837e1ec03c8fe1ac3c8d09fd374fb8b0e02",
            "git_hash_algorithm": "sha1",
            "branch": "development"
        },
        "producer": {
            "declared_by": "Developer",
            "role": "developer"
        },
        "ci_evidence": {
            "workflow_path": ".github/workflows/ci.yml",
            "run_id": "40144852713",
            "run_attempt": 1,
            "expected_policy_ref": "docs/gov-00/policies/standard-ci-policy-v1.json",
            "expected_policy_digest": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            "expected_jobs": ["build", "lint", "queue-task"],
            "observed_jobs": [
                { "name": "build", "status": "completed", "conclusion": "success" },
                { "name": "lint", "status": "completed", "conclusion": "failure" },
                { "name": "queue-task", "status": "queued" }
            ]
        },
        "integrity": {
            "canonicalization_algorithm": "RFC8785",
            "digest_algorithm": "sha256"
        }
    });
    assert!(store.validate_payload(&payload).is_ok());
}

#[test]
fn test_inspect_untrusted_payload_diagnostics_only() {
    let bytes = load_fixture("input_valid_bootstrap_checkpoint.json");
    let output = inspect_payload(&bytes, &schema_dir());

    assert_eq!(output.exit_code, 3);
    assert_eq!(
        output.assessment.pointer("/derived_summary/verdict").unwrap(),
        "REFUSED"
    );
    assert_eq!(
        output
            .assessment
            .pointer("/dimensions/evidence_integrity/digest_match")
            .unwrap(),
        false
    );
}

#[test]
fn test_valid_synthetic_assessment_handled_as_synthetic() {
    let store = SchemaStore::load(&schema_dir()).expect("schemas should load");
    let bytes = load_fixture("output_valid_provisional_assessment.json");
    let assessment: Value = serde_json::from_slice(&bytes).unwrap();

    assert!(store.validate_assessment(&assessment).is_ok());
    assert_eq!(
        assessment.pointer("/evaluator_auth/is_synthetic_mock").unwrap(),
        true
    );
    assert_eq!(
        assessment
            .pointer("/evaluator_auth/execution_context")
            .unwrap(),
        "SYNTHETIC_MOCK_UNTRUSTED"
    );
    assert_eq!(
        assessment.pointer("/claims_breakdown/human_authorized/authorized").unwrap(),
        false
    );
}
