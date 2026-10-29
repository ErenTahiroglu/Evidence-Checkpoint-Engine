// integration_positive.rs — Positive integration tests for Evidence Checkpoint Engine.

use ece_lib::verifier::{
    build_assessment_refused, canonicalize, compute_digest, inspect_payload, verify_envelope,
    SchemaStore, VerifierError,
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
    std::fs::read(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()))
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
        output
            .assessment
            .pointer("/derived_summary/verdict")
            .unwrap(),
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
        output
            .assessment
            .pointer("/derived_summary/verdict")
            .unwrap(),
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
        assessment
            .pointer("/evaluator_auth/is_synthetic_mock")
            .unwrap(),
        true
    );
    assert_eq!(
        assessment
            .pointer("/evaluator_auth/execution_context")
            .unwrap(),
        "SYNTHETIC_MOCK_UNTRUSTED"
    );
    assert_eq!(
        assessment
            .pointer("/claims_breakdown/human_authorized/authorized")
            .unwrap(),
        false
    );
}

#[test]
fn test_rfc8785_ecmascript_number_vectors() {
    let cases: Vec<(Value, &[u8])> = vec![
        (json!(1.0), b"1"),
        (json!(0.0), b"0"),
        (json!(-0.0), b"0"),
        (json!(1e30), b"1e+30"),
        (json!(2e-3), b"0.002"),
        (json!(1e-6), b"0.000001"),
        (json!(1e-7), b"1e-7"),
        (json!(1e20), b"100000000000000000000"),
        (json!(1e21), b"1e+21"),
        (json!(5e-324), b"5e-324"),
        (json!(-1.5), b"-1.5"),
    ];

    for (val, expected) in cases {
        let actual = canonicalize(&val).expect("canonicalization must succeed");
        assert_eq!(
            actual,
            expected,
            "mismatch for value: {}, expected: {}, got: {}",
            val,
            std::str::from_utf8(expected).unwrap(),
            std::str::from_utf8(&actual).unwrap()
        );
    }
}

#[test]
fn test_emitted_assessment_valid_envelope_exit_1_conforms_to_schema() {
    let store = SchemaStore::load(&schema_dir()).expect("schemas should load");
    let bytes = load_fixture("envelope_valid_bootstrap_checkpoint.json");
    let output = verify_envelope(&bytes, &schema_dir());

    assert_eq!(output.exit_code, 1);
    store
        .validate_assessment(&output.assessment)
        .expect("emitted exit 1 assessment must conform to assessment-output.schema.json");
}

#[test]
fn test_emitted_assessment_digest_mismatch_exit_2_conforms_to_schema() {
    let store = SchemaStore::load(&schema_dir()).expect("schemas should load");
    let bytes = load_fixture("envelope_invalid_tampered_digest.json");
    let output = verify_envelope(&bytes, &schema_dir());

    assert_eq!(output.exit_code, 2);
    store
        .validate_assessment(&output.assessment)
        .expect("emitted exit 2 assessment must conform to assessment-output.schema.json");
}

#[test]
fn test_emitted_assessment_refused_syntax_error_exit_3_conforms_to_schema() {
    let store = SchemaStore::load(&schema_dir()).expect("schemas should load");
    let bytes = load_fixture("input_malformed_syntax.txt");
    let output = verify_envelope(&bytes, &schema_dir());

    assert_eq!(output.exit_code, 3);
    assert_eq!(
        output.assessment.pointer("/evaluated_target/repository_id"),
        Some(&Value::Null),
        "refused assessment must not fabricate fake repository_id"
    );
    assert_eq!(
        output
            .assessment
            .pointer("/evaluated_target/target_commit_sha"),
        Some(&Value::Null),
        "refused assessment must not fabricate fake target_commit_sha"
    );
    store
        .validate_assessment(&output.assessment)
        .expect("emitted exit 3 refused assessment must conform to assessment-output.schema.json");
}

#[test]
fn test_emitted_assessment_inspect_untrusted_exit_3_conforms_to_schema() {
    let store = SchemaStore::load(&schema_dir()).expect("schemas should load");
    let bytes = load_fixture("input_valid_bootstrap_checkpoint.json");
    let output = inspect_payload(&bytes, &schema_dir());

    assert_eq!(output.exit_code, 3);
    store.validate_assessment(&output.assessment).expect(
        "emitted exit 3 inspect-untrusted assessment must conform to assessment-output.schema.json",
    );
}

#[test]
fn test_emitted_assessment_operational_error_exit_4_conforms_to_schema() {
    let store = SchemaStore::load(&schema_dir()).expect("schemas should load");
    let op_error = VerifierError::SchemaIo("schema file missing".to_owned());
    assert_eq!(op_error.exit_code(), 4);
    let output = build_assessment_refused(&op_error, None);

    assert_eq!(output.exit_code, 4);
    assert_eq!(
        output.assessment.pointer("/derived_summary/verdict"),
        Some(&json!("REFUSED"))
    );
    assert_eq!(
        output.assessment.pointer("/evaluated_target/repository_id"),
        Some(&Value::Null),
        "operational error assessment must not fabricate fake repository_id"
    );
    store.validate_assessment(&output.assessment).expect(
        "emitted exit 4 operational error assessment must conform to assessment-output.schema.json",
    );
}
