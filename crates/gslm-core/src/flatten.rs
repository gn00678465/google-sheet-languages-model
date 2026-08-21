use serde_json::{Map, Value};
use thiserror::Error;

/// Default key separator (see CONTEXT.md: Key separator).
pub const DEFAULT_SEPARATOR: &str = ".";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FlattenError {
    #[error("expected an object at the top level, got {0}")]
    NotAnObject(&'static str),
    #[error("key segment must not be a number: {0:?}")]
    NumericKeySegment(String),
    #[error("separator must not be empty")]
    EmptySeparator,
    #[error("arrays are not supported in catalogs (at key {0:?})")]
    ArrayNotSupported(String),
}

/// Flatten a nested catalog into a single-level map whose keys are joined by
/// `separator`. Key order of the input is preserved (depth-first).
pub fn flatten(value: &Value, separator: &str) -> Result<Map<String, Value>, FlattenError> {
    if separator.is_empty() {
        return Err(FlattenError::EmptySeparator);
    }
    let Value::Object(root) = value else {
        return Err(FlattenError::NotAnObject(type_name(value)));
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
) -> Result<(), FlattenError> {
    for (key, child) in obj {
        if is_numeric(key) {
            return Err(FlattenError::NumericKeySegment(key.clone()));
        }
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}{separator}{key}")
        };
        match child {
            Value::Object(inner) => walk(inner, separator, path, out)?,
            Value::Array(_) => return Err(FlattenError::ArrayNotSupported(path)),
            leaf => {
                out.insert(path, leaf.clone());
            }
        }
    }
    Ok(())
}

/// Mirrors the legacy TS check `/^-?\d+$/`.
fn is_numeric(segment: &str) -> bool {
    let digits = segment.strip_prefix('-').unwrap_or(segment);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
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
        assert_eq!(err, FlattenError::NumericKeySegment("0".into()));
        let err = flatten(&json!({"-12": "x"}), ".").unwrap_err();
        assert_eq!(err, FlattenError::NumericKeySegment("-12".into()));
    }

    #[test]
    fn rejects_non_object_root() {
        assert_eq!(
            flatten(&json!(["a"]), ".").unwrap_err(),
            FlattenError::NotAnObject("array")
        );
        assert_eq!(
            flatten(&json!("s"), ".").unwrap_err(),
            FlattenError::NotAnObject("string")
        );
    }

    #[test]
    fn rejects_arrays_with_their_path() {
        // Legacy TS flattened arrays into `days.0`, `days.1`, which its own
        // unflatten then rejected as numeric keys; arrays never round-tripped.
        assert_eq!(
            flatten(&json!({"a": {"days": ["Mon", "Tue"]}}), ".").unwrap_err(),
            FlattenError::ArrayNotSupported("a.days".into())
        );
    }

    #[test]
    fn rejects_empty_separator() {
        assert_eq!(
            flatten(&json!({"a": "b"}), "").unwrap_err(),
            FlattenError::EmptySeparator
        );
    }
}
