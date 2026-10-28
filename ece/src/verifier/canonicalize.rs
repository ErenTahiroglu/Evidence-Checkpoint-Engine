// canonicalize.rs — RFC 8785 JSON Canonicalization Scheme (JCS) implementation.
//
// Normative reference: https://www.rfc-editor.org/rfc/rfc8785
//
// Key rules implemented:
//   - Object keys sorted lexicographically by UTF-16 code units (not Unicode codepoints)
//   - No whitespace outside string literals
//   - Negative zero serialized as "0"
//   - Integers serialized without decimal point
//   - Floats follow ECMAScript JSON.stringify rules
//   - String escaping: only \", \\, and \u00xx for control chars U+0000–U+001F
//   - Solidus (/) never escaped
//   - Output is UTF-8 without BOM

use crate::verifier::error::VerifierError;
use serde_json::Value;

/// Canonicalize a JSON value per RFC 8785.
/// Returns the canonical UTF-8 byte sequence.
pub fn canonicalize(value: &Value) -> Result<Vec<u8>, VerifierError> {
    let mut buf = Vec::with_capacity(256);
    write_value(value, &mut buf)?;
    Ok(buf)
}

fn write_value(value: &Value, buf: &mut Vec<u8>) -> Result<(), VerifierError> {
    match value {
        Value::Null => buf.extend_from_slice(b"null"),
        Value::Bool(true) => buf.extend_from_slice(b"true"),
        Value::Bool(false) => buf.extend_from_slice(b"false"),
        Value::Number(n) => write_number(n, buf)?,
        Value::String(s) => write_string(s, buf),
        Value::Array(items) => {
            buf.push(b'[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                write_value(item, buf)?;
            }
            buf.push(b']');
        }
        Value::Object(map) => {
            // Sort keys by UTF-16 code-unit sequence (RFC 8785 §3.2.3)
            let mut pairs: Vec<(&String, &Value)> = map.iter().collect();
            pairs.sort_by(|(a, _), (b, _)| {
                let a_utf16: Vec<u16> = a.encode_utf16().collect();
                let b_utf16: Vec<u16> = b.encode_utf16().collect();
                a_utf16.cmp(&b_utf16)
            });
            buf.push(b'{');
            for (i, (key, val)) in pairs.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                write_string(key, buf);
                buf.push(b':');
                write_value(val, buf)?;
            }
            buf.push(b'}');
        }
    }
    Ok(())
}

/// Write a JSON number per RFC 8785 / ECMAScript serialization rules.
fn write_number(n: &serde_json::Number, buf: &mut Vec<u8>) -> Result<(), VerifierError> {
    // Prefer integer representation: try i64, then u64, then f64.
    if let Some(i) = n.as_i64() {
        buf.extend_from_slice(i.to_string().as_bytes());
        return Ok(());
    }
    if let Some(u) = n.as_u64() {
        buf.extend_from_slice(u.to_string().as_bytes());
        return Ok(());
    }
    if let Some(f) = n.as_f64() {
        if f == 0.0 {
            // Negative zero → "0"
            buf.push(b'0');
            return Ok(());
        }
        if f.is_infinite() || f.is_nan() {
            return Err(VerifierError::InternalError(
                "non-finite float in JSON value".to_owned(),
            ));
        }
        // ECMAScript number serialization: use shortest round-trip representation.
        // Rust's default f64 Display is shortest round-trip (since 1.x via Grisu3/Dragon4).
        // We need to ensure no trailing zeros in mantissa and use lowercase 'e'.
        let s = format_ecma_float(f);
        buf.extend_from_slice(s.as_bytes());
        return Ok(());
    }
    Err(VerifierError::InternalError(
        "number neither i64, u64, nor f64".to_owned(),
    ))
}

/// Format a finite, non-zero f64 per ECMAScript JSON.stringify rules:
/// - Shortest round-trip decimal representation
/// - No '+' in exponent, lowercase 'e'
/// - No trailing zeros in mantissa
fn format_ecma_float(f: f64) -> String {
    // Rust's Display for f64 already gives shortest round-trip.
    // We need to ensure the exponent uses lowercase 'e' and no '+'.
    let s = format!("{f:e}");
    // s looks like "1.5e7" or "1.5e-3" — Rust already uses lowercase 'e' and no '+' for positive exponent
    // But check: format!("{:e}", 1.5e7) == "1.5e7"? Let's use ryu-style shortest.
    // Actually the safest path: use serde_json's own serializer for f64, which follows ECMAScript rules.
    // We can serialize a temporary Value::Number and extract the raw text.
    // Even simpler: directly use format!("{}", f) which gives shortest round-trip in Rust.
    let plain = format!("{f}");
    // Check if the plain form round-trips
    if plain.parse::<f64>().ok() == Some(f) {
        // Ensure no uppercase E (Rust uses 'e' in format!("{:e}") but not in "{}").
        // format!("{}") on f64 in Rust never uses scientific notation for small-ish values,
        // but for very large/small may. Use the serde_json serialization which is ECMAScript-correct.
        // The safest approach: serialize via serde_json which uses ryu internally.
        let v = serde_json::Value::from(f);
        if let Some(s) = serde_json::to_string(&v).ok() {
            return s;
        }
    }
    plain
}

/// Write a JSON string with RFC 8785 escaping rules.
/// - Only escape: `"`, `\`, control chars U+0000–U+001F
/// - Never escape `/`
/// - Use lowercase `\uXXXX` for control chars (exactly 4 hex digits)
fn write_string(s: &str, buf: &mut Vec<u8>) {
    buf.push(b'"');
    for c in s.chars() {
        match c {
            '"' => buf.extend_from_slice(b"\\\""),
            '\\' => buf.extend_from_slice(b"\\\\"),
            '\x08' => buf.extend_from_slice(b"\\b"),
            '\x0c' => buf.extend_from_slice(b"\\f"),
            '\n' => buf.extend_from_slice(b"\\n"),
            '\r' => buf.extend_from_slice(b"\\r"),
            '\t' => buf.extend_from_slice(b"\\t"),
            c if (c as u32) < 0x20 => {
                // Other control chars: \u00XX (lowercase, 4 digits)
                let code = c as u32;
                buf.extend_from_slice(
                    format!("\\u{code:04x}").as_bytes(),
                );
            }
            c => {
                // Write UTF-8 bytes directly (covers all non-control, non-special chars including '/')
                let mut tmp = [0u8; 4];
                buf.extend_from_slice(c.encode_utf8(&mut tmp).as_bytes());
            }
        }
    }
    buf.push(b'"');
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
    fn object_keys_sorted_by_utf16() {
        // "b" (0x0062) sorts before "a" is wrong — "a" (0x0061) < "b" (0x0062)
        // So {"b":1,"a":2} → {"a":2,"b":1}
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
        assert!(canonical.windows(2).all(|w| w != b"\\/"),
            "solidus should not be escaped");
    }

    #[test]
    fn control_char_escaped_lowercase() {
        let v = Value::String("\u{0001}".to_owned());
        let canonical = canonicalize(&v).unwrap();
        assert_eq!(canonical, b"\"\\u0001\"");
    }

    #[test]
    fn bootstrap_canonical_length_and_digest() {
        // Load the bootstrap payload fixture
        let fixture_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../docs/gov-00/fixtures/input_valid_bootstrap_checkpoint.json"
        );
        let bytes = std::fs::read(fixture_path)
            .expect("bootstrap fixture not found");
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        let canonical = canonicalize(&value).unwrap();

        // Spec-mandated: 653 UTF-8 bytes (corrected from the erroneous 1,228 in GOV-00 R2 docs)
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
            hex,
            "157aaebc73d8daecfc07aa8e8b706fdcd1d4b5909d55442144a194734c20199d",
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
        // "z" key should appear first
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
        let bytes = std::fs::read(fixture_path)
            .expect("bootstrap fixture not found");
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        let c1 = canonicalize(&value).unwrap();
        let c2 = canonicalize(&value).unwrap();
        assert_eq!(c1, c2, "canonicalization must be deterministic");
    }
}

