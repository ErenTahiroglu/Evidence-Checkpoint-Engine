// digest.rs — SHA-256 digest computation and envelope digest verification.

use crate::verifier::{
    canonicalize::canonicalize,
    error::VerifierError,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Compute `sha256:<hex>` over canonical bytes.
pub fn compute_digest(canonical_bytes: &[u8]) -> String {
    let hash = Sha256::digest(canonical_bytes);
    format!("sha256:{}", hex::encode(hash))
}

/// Verify the envelope digest:
/// 1. Extract `payload` from envelope.
/// 2. Canonicalize payload (RFC 8785).
/// 3. Compute SHA-256.
/// 4. Compare to `envelope_manifest_digest`.
///
/// Returns the computed digest string on success, or DigestMismatch on failure.
/// The preimage is ONLY the `payload` object — outer envelope fields are excluded.
pub fn verify_envelope_digest(envelope: &Value) -> Result<String, VerifierError> {
    let payload = envelope.get("payload").ok_or_else(|| {
        VerifierError::InternalError("envelope missing 'payload' field".to_owned())
    })?;

    let canonical = canonicalize(payload)?;
    let computed = compute_digest(&canonical);

    let expected = envelope
        .get("envelope_manifest_digest")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            VerifierError::InternalError(
                "envelope missing 'envelope_manifest_digest' field".to_owned(),
            )
        })?;

    if computed != expected {
        return Err(VerifierError::DigestMismatch {
            expected: expected.to_owned(),
            computed,
        });
    }

    Ok(computed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn bootstrap_envelope_digest_matches() {
        let fixture_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../docs/gov-00/fixtures/envelope_valid_bootstrap_checkpoint.json"
        );
        let bytes = std::fs::read(fixture_path).expect("fixture not found");
        let envelope: Value = serde_json::from_slice(&bytes).unwrap();
        let result = verify_envelope_digest(&envelope);
        assert!(
            result.is_ok(),
            "bootstrap envelope digest should verify: {result:?}"
        );
        assert_eq!(
            result.unwrap(),
            "sha256:157aaebc73d8daecfc07aa8e8b706fdcd1d4b5909d55442144a194734c20199d"
        );
    }

    #[test]
    fn tampered_digest_rejected() {
        let fixture_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../docs/gov-00/fixtures/envelope_invalid_tampered_digest.json"
        );
        let bytes = std::fs::read(fixture_path).expect("fixture not found");
        let envelope: Value = serde_json::from_slice(&bytes).unwrap();
        let result = verify_envelope_digest(&envelope);
        assert!(
            matches!(result, Err(VerifierError::DigestMismatch { .. })),
            "tampered digest should fail: {result:?}"
        );
    }

    #[test]
    fn modified_payload_rejected() {
        // Build an envelope with the correct digest but then modify the payload
        let mut envelope = json!({
            "envelope_version": "0.1.0-draft",
            "payload": {
                "schema_version": "0.1.0-draft",
                "checkpoint_id": "test-001",
                "phase": "TEST-00",
                "timestamp": "2026-10-25T00:00:00Z",
                "repository": {"owner": "A", "name": "B", "github_rest_id": "123"},
                "source": {"commit_sha": "bead6837e1ec03c8fe1ac3c8d09fd374fb8b0e02", "git_hash_algorithm": "sha1", "branch": "main"},
                "producer": {"declared_by": "Test", "role": "developer"},
                "integrity": {"canonicalization_algorithm": "RFC8785", "digest_algorithm": "sha256"}
            },
            "envelope_manifest_digest": "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
        });
        // The digest above is wrong for the payload — should fail
        let result = verify_envelope_digest(&envelope);
        assert!(
            matches!(result, Err(VerifierError::DigestMismatch { .. })),
            "wrong digest should fail: {result:?}"
        );

        // Now compute the correct digest and verify it passes
        let payload = envelope["payload"].clone();
        let canonical = canonicalize(&payload).unwrap();
        let correct_digest = compute_digest(&canonical);
        envelope["envelope_manifest_digest"] = json!(correct_digest);
        let result2 = verify_envelope_digest(&envelope);
        assert!(result2.is_ok(), "correct digest should pass: {result2:?}");
    }
}

