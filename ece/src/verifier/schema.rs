// schema.rs — JSON Schema Draft 2020-12 validation with offline local $ref resolution.
//
// Security properties:
//   - All schemas loaded from disk at startup, keyed by their $id URI
//   - A custom Retrieve implementation maps $id URIs to local schema values
//   - No network access; unresolvable URIs are an error, not a network call
//   - No schema from the untrusted input document is ever used for validation

use crate::verifier::error::VerifierError;
use jsonschema::{Draft, Retrieve, Uri, Validator};
use serde_json::Value;
use std::{collections::HashMap, path::Path};

/// Holds compiled validators for the three schema types.
pub struct SchemaStore {
    pub envelope_validator: Validator,
    pub payload_validator: Validator,
    pub assessment_validator: Validator,
}

/// A local-only schema retriever that serves schemas from an in-memory map.
/// Prevents any network access during $ref resolution.
#[derive(Clone)]
struct LocalRetriever {
    schemas: HashMap<String, Value>,
}

impl Retrieve for LocalRetriever {
    fn retrieve(
        &self,
        uri: &Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let uri_str = uri.as_str();
        self.schemas
            .get(uri_str)
            .cloned()
            .ok_or_else(|| format!("Local schema not found for URI: {uri_str}").into())
    }
}

impl SchemaStore {
    /// Load and compile all three schemas from `schema_dir`.
    ///
    /// Schemas are registered by their `$id` so that cross-schema `$ref`
    /// resolution (envelope → payload) works entirely offline.
    pub fn load(schema_dir: &Path) -> Result<Self, VerifierError> {
        let input_path = schema_dir.join("checkpoint-input.schema.json");
        let envelope_path = schema_dir.join("checkpoint-envelope.schema.json");
        let assessment_path = schema_dir.join("assessment-output.schema.json");

        let input_text = std::fs::read_to_string(&input_path).map_err(|e| {
            VerifierError::SchemaIo(format!("cannot read {}: {e}", input_path.display()))
        })?;
        let envelope_text = std::fs::read_to_string(&envelope_path).map_err(|e| {
            VerifierError::SchemaIo(format!("cannot read {}: {e}", envelope_path.display()))
        })?;
        let assessment_text = std::fs::read_to_string(&assessment_path).map_err(|e| {
            VerifierError::SchemaIo(format!("cannot read {}: {e}", assessment_path.display()))
        })?;

        let input_schema: Value = serde_json::from_str(&input_text).map_err(|e| {
            VerifierError::SchemaIo(format!("cannot parse checkpoint-input schema: {e}"))
        })?;
        let envelope_schema: Value =
            serde_json::from_str(&envelope_text).map_err(|e| {
                VerifierError::SchemaIo(format!("cannot parse checkpoint-envelope schema: {e}"))
            })?;
        let assessment_schema: Value =
            serde_json::from_str(&assessment_text).map_err(|e| {
                VerifierError::SchemaIo(format!("cannot parse assessment-output schema: {e}"))
            })?;

        // Extract $id values for registration
        let input_id = schema_id(&input_schema, "checkpoint-input")?;
        let envelope_id = schema_id(&envelope_schema, "checkpoint-envelope")?;
        let assessment_id = schema_id(&assessment_schema, "assessment-output")?;

        // Build the local retriever map — both schemas registered so $ref resolution works
        let mut schema_map: HashMap<String, Value> = HashMap::new();
        schema_map.insert(input_id.clone(), input_schema.clone());
        schema_map.insert(envelope_id.clone(), envelope_schema.clone());
        schema_map.insert(assessment_id.clone(), assessment_schema.clone());
        // Also register by filename (relative $ref may use just the filename)
        schema_map.insert("checkpoint-input.schema.json".to_owned(), input_schema.clone());
        schema_map.insert("checkpoint-envelope.schema.json".to_owned(), envelope_schema.clone());
        schema_map.insert("assessment-output.schema.json".to_owned(), assessment_schema.clone());

        let retriever = LocalRetriever {
            schemas: schema_map,
        };

        let payload_validator = build_validator(&input_schema, retriever.clone())
            .map_err(|e| VerifierError::SchemaIo(format!("cannot compile payload schema: {e}")))?;

        let envelope_validator = build_validator(&envelope_schema, retriever.clone())
            .map_err(|e| VerifierError::SchemaIo(format!("cannot compile envelope schema: {e}")))?;

        let assessment_validator = build_validator(&assessment_schema, retriever)
            .map_err(|e| {
                VerifierError::SchemaIo(format!("cannot compile assessment schema: {e}"))
            })?;

        Ok(SchemaStore {
            envelope_validator,
            payload_validator,
            assessment_validator,
        })
    }

    /// Validate a payload document against `checkpoint-input.schema.json`.
    pub fn validate_payload(&self, payload: &Value) -> Result<(), VerifierError> {
        collect_errors(&self.payload_validator, payload)
    }

    /// Validate an envelope document against `checkpoint-envelope.schema.json`.
    pub fn validate_envelope(&self, envelope: &Value) -> Result<(), VerifierError> {
        collect_errors(&self.envelope_validator, envelope)
    }

    /// Validate an assessment output document against `assessment-output.schema.json`.
    pub fn validate_assessment(&self, assessment: &Value) -> Result<(), VerifierError> {
        collect_errors(&self.assessment_validator, assessment)
    }
}

fn build_validator(
    schema: &Value,
    retriever: LocalRetriever,
) -> Result<Validator, Box<dyn std::error::Error>> {
    let validator = jsonschema::options()
        .with_draft(Draft::Draft202012)
        .with_retriever(retriever)
        .build(schema)?;
    Ok(validator)
}

fn schema_id(schema: &Value, fallback: &str) -> Result<String, VerifierError> {
    Ok(schema
        .get("$id")
        .and_then(|v| v.as_str())
        .unwrap_or(fallback)
        .to_owned())
}

fn collect_errors(validator: &Validator, instance: &Value) -> Result<(), VerifierError> {
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| format!("{} [path: {}]", e, e.instance_path))
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(VerifierError::SchemaViolation { details: errors })
    }
}

/// Default schema directory: `docs/gov-00/schemas/` relative to the current working directory.
pub fn default_schema_dir() -> std::path::PathBuf {
    std::path::PathBuf::from("docs/gov-00/schemas")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_schema_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../docs/gov-00/schemas"
        ))
    }

    fn load_store() -> SchemaStore {
        SchemaStore::load(&test_schema_dir()).expect("schema dir should load")
    }

    #[test]
    fn loads_schemas_without_network() {
        // This test must complete without making any network requests.
        let _ = load_store();
    }

    #[test]
    fn valid_bootstrap_payload_accepted() {
        let store = load_store();
        let fixture = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../docs/gov-00/fixtures/input_valid_bootstrap_checkpoint.json"
        ))
        .unwrap();
        let payload: Value = serde_json::from_slice(&fixture).unwrap();
        assert!(
            store.validate_payload(&payload).is_ok(),
            "bootstrap payload should be valid"
        );
    }

    #[test]
    fn valid_bootstrap_envelope_accepted() {
        let store = load_store();
        let fixture = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../docs/gov-00/fixtures/envelope_valid_bootstrap_checkpoint.json"
        ))
        .unwrap();
        let envelope: Value = serde_json::from_slice(&fixture).unwrap();
        assert!(
            store.validate_envelope(&envelope).is_ok(),
            "bootstrap envelope should be valid"
        );
    }

    #[test]
    fn unsupported_schema_version_rejected() {
        let store = load_store();
        let fixture = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../docs/gov-00/fixtures/input_invalid_unsupported_schema_version.json"
        ))
        .unwrap();
        let payload: Value = serde_json::from_slice(&fixture).unwrap();
        assert!(
            store.validate_payload(&payload).is_err(),
            "unsupported schema version should fail schema validation"
        );
    }

    #[test]
    fn sha1_algorithm_with_sha256_length_rejected() {
        let store = load_store();
        let fixture = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../docs/gov-00/fixtures/input_invalid_git_sha1_with_sha256_length.json"
        ))
        .unwrap();
        let payload: Value = serde_json::from_slice(&fixture).unwrap();
        // Schema's conditional if/then should reject this
        assert!(
            store.validate_payload(&payload).is_err(),
            "sha1 algorithm with 64-char hash should fail"
        );
    }

    #[test]
    fn sha256_algorithm_with_sha1_length_rejected() {
        let store = load_store();
        let fixture = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../docs/gov-00/fixtures/input_invalid_git_sha256_with_sha1_length.json"
        ))
        .unwrap();
        let payload: Value = serde_json::from_slice(&fixture).unwrap();
        assert!(
            store.validate_payload(&payload).is_err(),
            "sha256 algorithm with 40-char hash should fail"
        );
    }

    #[test]
    fn leading_zero_id_rejected() {
        let store = load_store();
        let fixture = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../docs/gov-00/fixtures/input_invalid_id_leading_zero.json"
        ))
        .unwrap();
        let payload: Value = serde_json::from_slice(&fixture).unwrap();
        assert!(
            store.validate_payload(&payload).is_err(),
            "leading-zero ID should fail pattern regex"
        );
    }

    #[test]
    fn self_injection_fixture_rejected_by_schema() {
        let store = load_store();
        let fixture = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../docs/gov-00/fixtures/input_invalid_self_accepted_injection.json"
        ))
        .unwrap();
        let payload: Value = serde_json::from_slice(&fixture).unwrap();
        // Has additionalProperties "verified", "accepted", "authoritative_status" → rejected
        assert!(
            store.validate_payload(&payload).is_err(),
            "self-injection payload with extra fields should fail additionalProperties"
        );
    }
}

