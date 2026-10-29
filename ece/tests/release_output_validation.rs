// release_output_validation.rs — Fail-closed verification tests for output schema validation.
//
// Regression test for GOV-01 R2: verifies that when an emitted assessment
// fails validation against assessment-output.schema.json, the engine
// fails closed in both debug and release configurations:
//   - Library functions return Err(VerifierError::OutputSchemaViolation { .. }) with exit code 4
//   - CLI exits with exit code 4, suppresses stdout, and prints diagnostic to stderr

use ece_lib::verifier::{inspect_payload, verify_envelope, VerifierError};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const INCOMPATIBLE_ASSESSMENT_SCHEMA: &str = r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://github.com/ErenTahiroglu/Evidence-Checkpoint-Engine/schemas/v0.1.0/assessment-output.schema.json",
  "type": "object",
  "required": ["mandatory_field_engine_will_never_emit"],
  "properties": {
    "mandatory_field_engine_will_never_emit": {
      "type": "string"
    }
  }
}"#;

struct TempSchemaDir {
    dir: PathBuf,
}

impl TempSchemaDir {
    fn new(incompatible_assessment_schema: &str) -> Self {
        let unique = format!(
            "ece_test_incompatible_schema_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let dir = std::env::temp_dir().join(unique);
        fs::create_dir_all(&dir).expect("create temp schema dir");

        let base_schema_dir = PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../docs/gov-00/schemas"
        ));

        fs::copy(
            base_schema_dir.join("checkpoint-input.schema.json"),
            dir.join("checkpoint-input.schema.json"),
        )
        .expect("copy checkpoint-input.schema.json");

        fs::copy(
            base_schema_dir.join("checkpoint-envelope.schema.json"),
            dir.join("checkpoint-envelope.schema.json"),
        )
        .expect("copy checkpoint-envelope.schema.json");

        fs::write(
            dir.join("assessment-output.schema.json"),
            incompatible_assessment_schema,
        )
        .expect("write incompatible assessment-output.schema.json");

        Self { dir }
    }

    fn path(&self) -> &Path {
        &self.dir
    }
}

impl Drop for TempSchemaDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../docs/gov-00/fixtures"
    ))
    .join(name)
}

#[test]
fn test_library_verify_envelope_fails_closed_on_incompatible_output_schema() {
    let temp_schemas = TempSchemaDir::new(INCOMPATIBLE_ASSESSMENT_SCHEMA);
    let envelope_bytes =
        fs::read(fixture_path("envelope_valid_bootstrap_checkpoint.json")).expect("read fixture");

    let result = verify_envelope(&envelope_bytes, temp_schemas.path());
    match result {
        Err(VerifierError::OutputSchemaViolation { details }) => {
            assert!(!details.is_empty(), "expected schema violation details");
            assert_eq!(
                result.unwrap_err().exit_code(),
                4,
                "output schema violation must yield exit code 4"
            );
        }
        Ok(_) => panic!("verify_envelope must not succeed with incompatible assessment schema"),
        Err(other) => panic!("expected OutputSchemaViolation, got: {other:?}"),
    }
}

#[test]
fn test_library_inspect_payload_fails_closed_on_incompatible_output_schema() {
    let temp_schemas = TempSchemaDir::new(INCOMPATIBLE_ASSESSMENT_SCHEMA);
    let payload_bytes =
        fs::read(fixture_path("input_valid_bootstrap_checkpoint.json")).expect("read fixture");

    let result = inspect_payload(&payload_bytes, temp_schemas.path());
    match result {
        Err(VerifierError::OutputSchemaViolation { details }) => {
            assert!(!details.is_empty(), "expected schema violation details");
            assert_eq!(
                result.unwrap_err().exit_code(),
                4,
                "output schema violation must yield exit code 4"
            );
        }
        Ok(_) => panic!("inspect_payload must not succeed with incompatible assessment schema"),
        Err(other) => panic!("expected OutputSchemaViolation, got: {other:?}"),
    }
}

#[test]
fn test_cli_verify_envelope_fails_closed_with_empty_stdout_and_exit_4() {
    let temp_schemas = TempSchemaDir::new(INCOMPATIBLE_ASSESSMENT_SCHEMA);
    let fixture = fixture_path("envelope_valid_bootstrap_checkpoint.json");

    let output = Command::new(env!("CARGO_BIN_EXE_ece"))
        .args([
            "verify-envelope",
            "--input",
            fixture.to_str().unwrap(),
            "--schema-dir",
            temp_schemas.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute ece binary");

    assert_eq!(
        output.status.code(),
        Some(4),
        "CLI must exit with code 4 on output schema violation"
    );

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout_str.trim().is_empty(),
        "CLI stdout must be empty on output schema violation, got: {stdout_str}"
    );

    let stderr_str = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr_str.contains("OUTPUT_SCHEMA_VIOLATION"),
        "stderr must identify OUTPUT_SCHEMA_VIOLATION, got: {stderr_str}"
    );
    assert!(
        stderr_str.contains("ECE_OPERATIONAL_ERROR"),
        "stderr must format operational error prefix, got: {stderr_str}"
    );
}

#[test]
fn test_cli_inspect_payload_fails_closed_with_empty_stdout_and_exit_4() {
    let temp_schemas = TempSchemaDir::new(INCOMPATIBLE_ASSESSMENT_SCHEMA);
    let fixture = fixture_path("input_valid_bootstrap_checkpoint.json");

    let output = Command::new(env!("CARGO_BIN_EXE_ece"))
        .args([
            "inspect-untrusted-payload",
            "--input",
            fixture.to_str().unwrap(),
            "--schema-dir",
            temp_schemas.path().to_str().unwrap(),
        ])
        .output()
        .expect("failed to execute ece binary");

    assert_eq!(
        output.status.code(),
        Some(4),
        "CLI must exit with code 4 on output schema violation"
    );

    let stdout_str = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout_str.trim().is_empty(),
        "CLI stdout must be empty on output schema violation, got: {stdout_str}"
    );

    let stderr_str = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr_str.contains("OUTPUT_SCHEMA_VIOLATION"),
        "stderr must identify OUTPUT_SCHEMA_VIOLATION, got: {stderr_str}"
    );
    assert!(
        stderr_str.contains("ECE_OPERATIONAL_ERROR"),
        "stderr must format operational error prefix, got: {stderr_str}"
    );
}
