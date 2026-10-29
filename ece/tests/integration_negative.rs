// integration_negative.rs — Negative integration tests for Evidence Checkpoint Engine.

use ece_lib::verifier::{
    parse_bounded_json, verify_envelope, ErrorCode, SchemaStore, VerifierError, MAX_INPUT_BYTES,
};
use serde_json::Value;
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
fn test_tampered_envelope_digest_rejected() {
    let bytes = load_fixture("envelope_invalid_tampered_digest.json");
    let output = verify_envelope(&bytes, &schema_dir()).expect("verification should produce output");

    assert_eq!(output.exit_code, 2);
    assert_eq!(
        output
            .assessment
            .pointer("/derived_summary/verdict")
            .unwrap(),
        "ASSESSED_FAIL"
    );
    assert_eq!(
        output
            .assessment
            .pointer("/dimensions/evidence_integrity/status")
            .unwrap(),
        "TAMPERED"
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
fn test_invalid_schema_version_rejected() {
    let bytes = load_fixture("input_invalid_unsupported_schema_version.json");
    let store = SchemaStore::load(&schema_dir()).unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(store.validate_payload(&val).is_err());
}

#[test]
fn test_malformed_json_syntax_rejected() {
    let bytes = load_fixture("input_malformed_syntax.txt");
    match parse_bounded_json(&bytes) {
        Err(VerifierError::MalformedJson { .. }) => {}
        other => panic!("expected MalformedJson, got {other:?}"),
    }

    let output = verify_envelope(&bytes, &schema_dir()).expect("verification should produce output");
    assert_eq!(output.exit_code, 3);
    assert_eq!(
        output
            .assessment
            .pointer("/derived_summary/verdict")
            .unwrap(),
        "REFUSED"
    );
}

#[test]
fn test_duplicate_json_keys_rejected() {
    let duplicate_json = br#"{"schema_version":"0.1.0-draft","dup":1,"dup":2}"#;
    match parse_bounded_json(duplicate_json) {
        Err(VerifierError::DuplicateKey { key }) => assert_eq!(key, "dup"),
        other => panic!("expected DuplicateKey, got {other:?}"),
    }
}

#[test]
fn test_invalid_utf8_rejected() {
    let bad_utf8 = b"\xFF\xFE\x00\x00";
    match parse_bounded_json(bad_utf8) {
        Err(VerifierError::InvalidUtf8) => {}
        other => panic!("expected InvalidUtf8, got {other:?}"),
    }
}

#[test]
fn test_excessive_input_size_rejected() {
    let oversized = vec![b' '; MAX_INPUT_BYTES + 10];
    match parse_bounded_json(&oversized) {
        Err(VerifierError::InputTooLarge) => {}
        other => panic!("expected InputTooLarge, got {other:?}"),
    }
}

#[test]
fn test_excessive_nesting_rejected() {
    let mut deep = String::new();
    for _ in 0..35 {
        deep.push('[');
    }
    deep.push('1');
    for _ in 0..35 {
        deep.push(']');
    }
    match parse_bounded_json(deep.as_bytes()) {
        Err(VerifierError::NestingTooDeep) => {}
        other => panic!("expected NestingTooDeep, got {other:?}"),
    }
}

#[test]
fn test_git_sha1_with_sha256_length_rejected() {
    let bytes = load_fixture("input_invalid_git_sha1_with_sha256_length.json");
    let store = SchemaStore::load(&schema_dir()).unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(store.validate_payload(&val).is_err());
}

#[test]
fn test_git_sha256_with_sha1_length_rejected() {
    let bytes = load_fixture("input_invalid_git_sha256_with_sha1_length.json");
    let store = SchemaStore::load(&schema_dir()).unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(store.validate_payload(&val).is_err());
}

#[test]
fn test_uint64_overflow_edge_rejected() {
    let bytes = load_fixture("input_invalid_uint64_overflow_edge.json");
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    match ece_lib::verifier::run_semantic_checks(&val) {
        Err(VerifierError::SemanticError {
            code: ErrorCode::Uint64Overflow,
            ..
        }) => {}
        other => panic!("expected Uint64Overflow, got {other:?}"),
    }
}

#[test]
fn test_uint64_twenty_nines_rejected() {
    let bytes = load_fixture("input_invalid_uint64_twenty_nines.json");
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    match ece_lib::verifier::run_semantic_checks(&val) {
        Err(VerifierError::SemanticError {
            code: ErrorCode::Uint64Overflow,
            ..
        }) => {}
        other => panic!("expected Uint64Overflow, got {other:?}"),
    }
}

#[test]
fn test_leading_zero_identifier_rejected() {
    let bytes = load_fixture("input_invalid_id_leading_zero.json");
    let store = SchemaStore::load(&schema_dir()).unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(store.validate_payload(&val).is_err());
}

#[test]
fn test_missing_expected_check_policy_rejected() {
    let bytes = load_fixture("input_invalid_ci_missing_policy.json");
    let store = SchemaStore::load(&schema_dir()).unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(store.validate_payload(&val).is_err());
}

#[test]
fn test_self_declared_acceptance_injection_rejected() {
    let bytes = load_fixture("input_invalid_self_accepted_injection.json");
    let store = SchemaStore::load(&schema_dir()).unwrap();
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    assert!(store.validate_payload(&val).is_err());
}

#[test]
fn test_synthetic_claiming_pass_or_human_auth_rejected() {
    let store = SchemaStore::load(&schema_dir()).unwrap();

    let bytes_pass = load_fixture("output_invalid_mock_claiming_pass.json");
    let val_pass: Value = serde_json::from_slice(&bytes_pass).unwrap();
    assert!(store.validate_assessment(&val_pass).is_err());

    let bytes_auth = load_fixture("output_invalid_synthetic_claiming_human_auth.json");
    let val_auth: Value = serde_json::from_slice(&bytes_auth).unwrap();
    assert!(store.validate_assessment(&val_auth).is_err());
}

#[test]
fn test_missing_expected_jobs_recorded() {
    let bytes = load_fixture("input_invalid_missing_ci_jobs.json");
    let val: Value = serde_json::from_slice(&bytes).unwrap();
    let res = ece_lib::verifier::run_semantic_checks(&val)
        .expect("semantic checks should pass with warning");
    assert!(!res.unresolved_jobs.is_empty());
}
