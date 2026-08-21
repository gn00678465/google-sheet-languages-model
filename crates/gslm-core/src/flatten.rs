use crate::error::{ConversionError, json_type_name};
use serde_json::{Map, Value};

/// Default key separator (see CONTEXT.md: Key separator).
pub const DEFAULT_SEPARATOR: &str = ".";

/// Flatten a nested catalog into a single-level map whose keys are joined by
/// `separator`. Key order of the input is preserved (depth-first).
pub fn flatten(value: &Value, separator: &str) -> Result<Map<String, Value>, ConversionError> {
    if separator.is_empty() {
        return Err(ConversionError::EmptySeparator);
    }
    let Value::Object(root) = value else {
        return Err(ConversionError::NotAnObject(json_type_name(value)));
    };
    let mut out = Map::new();
    walk(root, separator, String::new(), &mut out)?;
    Ok(out)
}

fn walk(
    obj: &Map<String, Value>,
    separator: &str,
    prefix: String,
    out: &mut Map<String, Value>,
) -> Result<(), ConversionError> {
    for (key, child) in obj {
        if is_numeric(key) {
            return Err(ConversionError::NumericKeySegment(key.clone()));
        }
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}{separator}{key}")
        };
        match child {
            Value::Object(inner) => walk(inner, separator, path, out)?,
            Value::Array(_) => return Err(ConversionError::ArrayNotSupported(path)),
            leaf => {
                out.insert(path, leaf.clone());
            }
        }
    }
    Ok(())
}

/// Mirrors the legacy TS check `/^-?\d+$/`.
pub(crate) fn is_numeric(segment: &str) -> bool {
    let digits = segment.strip_prefix('-').unwrap_or(segment);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn keys(m: &Map<String, Value>) -> Vec<&str> {
        m.keys().map(String::as_str).collect()
    }

    #[test]
    fn flattens_one_level() {
        let out = flatten(&json!({"a": {"b": "x"}, "c": "y"}), ".").unwrap();
        assert_eq!(Value::Object(out.clone()), json!({"a.b": "x", "c": "y"}));
        assert_eq!(keys(&out), ["a.b", "c"]);
    }

    #[test]
    fn flattens_deep_nesting_preserving_order() {
        let out = flatten(&json!({"z": {"y": {"x": "1"}, "w": "2"}, "a": "3"}), ".").unwrap();
        assert_eq!(keys(&out), ["z.y.x", "z.w", "a"]);
    }

    #[test]
    fn empty_object_gives_empty_map() {
        assert!(flatten(&json!({}), ".").unwrap().is_empty());
    }

    #[test]
    fn empty_nested_object_is_dropped() {
        let out = flatten(&json!({"a": {}, "b": "x"}), ".").unwrap();
        assert_eq!(keys(&out), ["b"]);
    }

    #[test]
    fn custom_separator() {
        let out = flatten(&json!({"a": {"b": "x"}}), "/").unwrap();
        assert_eq!(keys(&out), ["a/b"]);
    }

    #[test]
    fn non_object_leaves_are_kept_as_is() {
        let out = flatten(&json!({"a": {"n": 1, "t": true, "z": null}}), ".").unwrap();
        assert_eq!(out["a.n"], json!(1));
        assert_eq!(out["a.t"], json!(true));
        assert_eq!(out["a.z"], Value::Null);
    }

    #[test]
    fn rejects_numeric_key_segment() {
        let err = flatten(&json!({"a": {"0": "x"}}), ".").unwrap_err();
        assert_eq!(err, ConversionError::NumericKeySegment("0".into()));
        let err = flatten(&json!({"-12": "x"}), ".").unwrap_err();
        assert_eq!(err, ConversionError::NumericKeySegment("-12".into()));
    }

    #[test]
    fn rejects_non_object_root() {
        assert_eq!(
            flatten(&json!(["a"]), ".").unwrap_err(),
            ConversionError::NotAnObject("array")
        );
        assert_eq!(
            flatten(&json!("s"), ".").unwrap_err(),
            ConversionError::NotAnObject("string")
        );
    }

    #[test]
    fn rejects_arrays_with_their_path() {
        // Legacy TS flattened arrays into `days.0`, `days.1`, which its own
        // unflatten then rejected as numeric keys; arrays never round-tripped.
        assert_eq!(
            flatten(&json!({"a": {"days": ["Mon", "Tue"]}}), ".").unwrap_err(),
            ConversionError::ArrayNotSupported("a.days".into())
        );
    }

    #[test]
    fn rejects_empty_separator() {
        assert_eq!(
            flatten(&json!({"a": "b"}), "").unwrap_err(),
            ConversionError::EmptySeparator
        );
    }
}
