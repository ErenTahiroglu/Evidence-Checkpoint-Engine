// canonicalize.rs — RFC 8785 JSON Canonicalization Scheme (JCS) implementation.
//
// Normative reference: https://www.rfc-editor.org/rfc/rfc8785
//
// Powered by serde_jcs, which implements byte-exact RFC 8785 canonicalization
// including UTF-16 code unit key sorting and ECMA-262 7.1.12.1 number formatting
// (via ryu-js).

use crate::verifier::error::VerifierError;
use serde_json::Value;

/// Canonicalize a JSON value per RFC 8785.
/// Returns the canonical UTF-8 byte sequence.
pub fn canonicalize(value: &Value) -> Result<Vec<u8>, VerifierError> {
    serde_jcs::to_vec(value).map_err(|e| VerifierError::InternalError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn null_value() {
        assert_eq!(canonicalize(&Value::Null).unwrap(), b"null");
    }

    #[test]
    fn bool_values() {
        assert_eq!(canonicalize(&json!(true)).unwrap(), b"true");
        assert_eq!(canonicalize(&json!(false)).unwrap(), b"false");
    }

    #[test]
    fn integer_no_decimal_point() {
        assert_eq!(canonicalize(&json!(1)).unwrap(), b"1");
        assert_eq!(canonicalize(&json!(0)).unwrap(), b"0");
        assert_eq!(canonicalize(&json!(-1)).unwrap(), b"-1");
        assert_eq!(canonicalize(&json!(42)).unwrap(), b"42");
    }

    #[test]
    fn ecmascript_number_vectors() {
        // Test RFC 8785 / ECMA-262 7.1.12.1 float representation rules
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
            let actual = canonicalize(&val).unwrap();
            assert_eq!(
                actual,
                expected,
                "failed for input: {}, expected: {}, got: {}",
                val,
                std::str::from_utf8(expected).unwrap(),
                std::str::from_utf8(&actual).unwrap()
            );
        }
    }

    #[test]
    fn object_keys_sorted_by_utf16() {
        // "b" (0x0062) sorts after "a" (0x0061)
        // {"b":1,"a":2} → {"a":2,"b":1}
        let v = json!({"b": 1, "a": 2});
        let canonical = canonicalize(&v).unwrap();
        assert_eq!(canonical, br#"{"a":2,"b":1}"#);
    }

    #[test]
    fn no_whitespace_in_output() {
        let v = json!({"key": "value", "num": 42});
        let canonical = canonicalize(&v).unwrap();
        let s = std::str::from_utf8(&canonical).unwrap();
        assert!(!s.contains(' '), "found whitespace: {s}");
        assert!(!s.contains('\n'), "found newline: {s}");
    }

    #[test]
    fn solidus_not_escaped() {
        let v = json!({"url": "http://example.com/path"});
        let canonical = canonicalize(&v).unwrap();
        assert!(
            canonical.windows(2).all(|w| w != b"\\/"),
            "solidus should not be escaped"
        );
    }

    #[test]
    fn control_char_escaped_lowercase() {
        let v = Value::String("\u{0001}".to_owned());
        let canonical = canonicalize(&v).unwrap();
        assert_eq!(canonical, b"\"\\u0001\"");
    }

    #[test]
    fn bootstrap_canonical_length_and_digest() {
        let fixture_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../docs/gov-00/fixtures/input_valid_bootstrap_checkpoint.json"
        );
        let bytes = std::fs::read(fixture_path).expect("bootstrap fixture not found");
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        let canonical = canonicalize(&value).unwrap();

        // Spec-mandated: 653 UTF-8 bytes
        assert_eq!(
            canonical.len(),
            653,
            "canonical length must be 653 bytes, got {}",
            canonical.len()
        );

        // Spec-mandated digest
        use sha2::{Digest, Sha256};
        let hash = Sha256::digest(&canonical);
        let hex = hex::encode(hash);
        assert_eq!(
            hex, "157aaebc73d8daecfc07aa8e8b706fdcd1d4b5909d55442144a194734c20199d",
            "digest mismatch"
        );
    }

    #[test]
    fn non_bmp_key_utf16_sort() {
        // Emoji have high surrogate pairs in UTF-16.
        // "\u{1F600}" (U+1F600, GRINNING FACE) encodes as D83D DE00 in UTF-16.
        // "z" (U+007A) encodes as 007A.
        // 007A < D83D, so "z" should sort before the emoji key.
        let v = json!({"\u{1F600}": 1, "z": 2});
        let canonical = canonicalize(&v).unwrap();
        let s = std::str::from_utf8(&canonical).unwrap();
        let z_pos = s.find("\"z\"").unwrap();
        let emoji_pos = s.find("\u{1F600}").unwrap();
        assert!(z_pos < emoji_pos, "z should sort before emoji: {s}");
    }

    #[test]
    fn array_order_preserved() {
        let v = json!([3, 1, 2]);
        let canonical = canonicalize(&v).unwrap();
        assert_eq!(canonical, b"[3,1,2]");
    }

    #[test]
    fn nested_object_sorted() {
        let v = json!({"z": {"b": 1, "a": 2}, "a": 3});
        let canonical = canonicalize(&v).unwrap();
        assert_eq!(canonical, br#"{"a":3,"z":{"a":2,"b":1}}"#);
    }

    #[test]
    fn backslash_and_quote_escaped() {
        let v = Value::String("say \"hello\" and \\goodbye".to_owned());
        let canonical = canonicalize(&v).unwrap();
        assert_eq!(canonical, br#""say \"hello\" and \\goodbye""#);
    }

    #[test]
    fn deterministic_repeated_calls() {
        let fixture_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../docs/gov-00/fixtures/input_valid_bootstrap_checkpoint.json"
        );
        let bytes = std::fs::read(fixture_path).expect("bootstrap fixture not found");
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        let c1 = canonicalize(&value).unwrap();
        let c2 = canonicalize(&value).unwrap();
        assert_eq!(c1, c2, "canonicalization must be deterministic");
    }
}
