// parser.rs — Bounded, duplicate-key-detecting JSON parser.
//
// Security properties enforced here:
//   - Maximum input size: 10 MiB
//   - Valid UTF-8 (enforced before any parsing)
//   - Duplicate object keys rejected before canonicalization
//   - Maximum JSON nesting depth: 32
//   - No execution of embedded content

use crate::verifier::error::VerifierError;
use serde_json::Value;
use std::collections::HashSet;

/// Maximum allowed input size: 10 MiB.
pub const MAX_INPUT_BYTES: usize = 10 * 1024 * 1024;

/// Maximum allowed JSON nesting depth.
pub const MAX_NESTING_DEPTH: usize = 32;

/// Parse a JSON byte slice with all safety guards applied.
///
/// Steps (in order):
/// 1. Size check (<= 10 MiB)
/// 2. UTF-8 validation
/// 3. Duplicate-key detection via streaming parse
/// 4. Nesting depth check (<= 32)
pub fn parse_bounded_json(input: &[u8]) -> Result<Value, VerifierError> {
    // 1. Size guard
    if input.len() > MAX_INPUT_BYTES {
        return Err(VerifierError::InputTooLarge);
    }

    // 2. UTF-8 guard
    let text = std::str::from_utf8(input).map_err(|_| VerifierError::InvalidUtf8)?;

    // 3. Duplicate-key detection + parse
    let value = parse_checking_duplicates(text)?;

    // 4. Nesting depth guard
    check_depth(&value, 0)?;

    Ok(value)
}

/// Parse JSON text while detecting duplicate object keys.
/// Uses a custom recursive descent over serde_json's streaming deserializer.
fn parse_checking_duplicates(text: &str) -> Result<Value, VerifierError> {
    // We use serde_json's raw streaming deserializer to detect duplicates.
    // Strategy: deserialize into a custom structure that errors on dup keys.
    use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
    use serde_json::de::StrRead;
    use std::fmt;

    struct DupCheckVisitor;

    impl<'de> Visitor<'de> for DupCheckVisitor {
        type Value = Value;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "any valid JSON value")
        }

        fn visit_bool<E: serde::de::Error>(self, v: bool) -> Result<Value, E> {
            Ok(Value::Bool(v))
        }

        fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Value, E> {
            Ok(Value::Number(v.into()))
        }

        fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Value, E> {
            Ok(Value::Number(v.into()))
        }

        fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Value, E> {
            // Use serde_json's Number::from_f64 to preserve exact representation
            match serde_json::Number::from_f64(v) {
                Some(n) => Ok(Value::Number(n)),
                None => Err(E::custom("non-finite float")),
            }
        }

        fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Value, E> {
            Ok(Value::String(v.to_owned()))
        }

        fn visit_unit<E: serde::de::Error>(self) -> Result<Value, E> {
            Ok(Value::Null)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
            let mut items = Vec::new();
            while let Some(item) = seq.next_element_seed(DupCheckSeed)? {
                items.push(item);
            }
            Ok(Value::Array(items))
        }

        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
            let mut keys_seen = HashSet::new();
            let mut entries = serde_json::Map::new();
            while let Some(key) = map.next_key::<String>()? {
                if !keys_seen.insert(key.clone()) {
                    return Err(serde::de::Error::custom(format!(
                        "DUPLICATE_KEY_TAG:{key}:END_DUPLICATE_KEY_TAG"
                    )));
                }
                let value: Value = map.next_value_seed(DupCheckSeed)?;
                entries.insert(key, value);
            }
            Ok(Value::Object(entries))
        }
    }

    struct DupCheckSeed;

    impl<'de> serde::de::DeserializeSeed<'de> for DupCheckSeed {
        type Value = Value;

        fn deserialize<D: Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
            d.deserialize_any(DupCheckVisitor)
        }
    }

    let mut de = serde_json::Deserializer::new(StrRead::new(text));
    let value = DupCheckSeed.deserialize(&mut de).map_err(|e| {
        let msg = e.to_string();
        let tag_start = "DUPLICATE_KEY_TAG:";
        let tag_end = ":END_DUPLICATE_KEY_TAG";
        if let Some(start) = msg.find(tag_start) {
            let rest = &msg[start + tag_start.len()..];
            let key = if let Some(end) = rest.find(tag_end) {
                rest[..end].to_owned()
            } else {
                rest.to_owned()
            };
            VerifierError::DuplicateKey { key }
        } else {
            VerifierError::MalformedJson { detail: msg }
        }
    })?;

    // Ensure no trailing content after the JSON value
    de.end().map_err(|e| VerifierError::MalformedJson {
        detail: format!("trailing content: {e}"),
    })?;

    Ok(value)
}

/// Recursively check that nesting depth does not exceed MAX_NESTING_DEPTH.
fn check_depth(value: &Value, current_depth: usize) -> Result<(), VerifierError> {
    if current_depth > MAX_NESTING_DEPTH {
        return Err(VerifierError::NestingTooDeep);
    }
    match value {
        Value::Array(items) => {
            for item in items {
                check_depth(item, current_depth + 1)?;
            }
        }
        Value::Object(map) => {
            for v in map.values() {
                check_depth(v, current_depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_input_over_10mib() {
        let big = vec![b' '; MAX_INPUT_BYTES + 1];
        assert!(matches!(
            parse_bounded_json(&big),
            Err(VerifierError::InputTooLarge)
        ));
    }

    #[test]
    fn rejects_invalid_utf8() {
        let bad = b"\xff\xfe{}";
        assert!(matches!(
            parse_bounded_json(bad),
            Err(VerifierError::InvalidUtf8)
        ));
    }

    #[test]
    fn rejects_duplicate_keys() {
        let json = br#"{"a":1,"a":2}"#;
        match parse_bounded_json(json) {
            Err(VerifierError::DuplicateKey { key }) => assert_eq!(key, "a"),
            other => panic!("expected DuplicateKey, got {other:?}"),
        }
    }

    #[test]
    fn rejects_malformed_json() {
        let json = b"{not valid json";
        assert!(matches!(
            parse_bounded_json(json),
            Err(VerifierError::MalformedJson { .. })
        ));
    }

    #[test]
    fn rejects_nesting_too_deep() {
        // Build a 33-level deep array
        let mut json = String::new();
        for _ in 0..34 {
            json.push('[');
        }
        json.push('1');
        for _ in 0..34 {
            json.push(']');
        }
        assert!(matches!(
            parse_bounded_json(json.as_bytes()),
            Err(VerifierError::NestingTooDeep)
        ));
    }

    #[test]
    fn accepts_valid_json() {
        let json = br#"{"key": "value", "num": 42}"#;
        let v = parse_bounded_json(json).expect("valid JSON should parse");
        assert_eq!(v["key"], "value");
        assert_eq!(v["num"], 42);
    }

    #[test]
    fn accepts_nesting_at_limit() {
        // Exactly 32 levels — should be accepted
        let mut json = String::new();
        for _ in 0..32 {
            json.push('[');
        }
        json.push('1');
        for _ in 0..32 {
            json.push(']');
        }
        assert!(parse_bounded_json(json.as_bytes()).is_ok());
    }
}
