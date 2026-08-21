use crate::error::ConversionError;
use crate::flatten::is_numeric;
use serde_json::{Map, Value};

/// Inverse of [`crate::flatten`]: rebuild a nested object from single-level
/// keys joined by `separator`. Insertion order is preserved at every level.
///
/// Errors when a key would have to be both a leaf and a parent
/// (`"a"` together with `"a.b"`), when a segment is numeric, or when
/// `separator` is empty.
pub fn unflatten(flat: &Map<String, Value>, separator: &str) -> Result<Value, ConversionError> {
    if separator.is_empty() {
        return Err(ConversionError::EmptySeparator);
    }
    let mut root = Map::new();
    for (key, value) in flat {
        insert_path(&mut root, key, separator, value)?;
    }
    Ok(Value::Object(root))
}

fn insert_path(
    root: &mut Map<String, Value>,
    key: &str,
    separator: &str,
    value: &Value,
) -> Result<(), ConversionError> {
    let segments: Vec<&str> = key.split(separator).collect();
    if let Some(bad) = segments.iter().find(|s| is_numeric(s)) {
        return Err(ConversionError::NumericKeySegment((*bad).to_string()));
    }
    let (last, parents) = segments
        .split_last()
        .expect("split yields at least one segment");

    let mut node = root;
    for segment in parents {
        let entry = node
            .entry((*segment).to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        node = match entry {
            Value::Object(m) => m,
            // An existing leaf where we need a parent: `a` then `a.b`.
            _ => {
                return Err(ConversionError::KeyConflict {
                    key: key.to_string(),
                });
            }
        };
    }
    match node.get(*last) {
        // An existing parent where we need a leaf: `a.b` then `a`.
        Some(Value::Object(_)) => Err(ConversionError::KeyConflict {
            key: key.to_string(),
        }),
        _ => {
            node.insert((*last).to_string(), value.clone());
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn m(v: Value) -> Map<String, Value> {
        match v {
            Value::Object(m) => m,
            _ => unreachable!(),
        }
    }

    #[test]
    fn rebuilds_nested_structure() {
        let out = unflatten(&m(json!({"a.b": "x", "c": "y"})), ".").unwrap();
        assert_eq!(out, json!({"a": {"b": "x"}, "c": "y"}));
    }

    #[test]
    fn preserves_order_at_every_level() {
        let out = unflatten(&m(json!({"z.y": "1", "z.x": "2", "a": "3"})), ".").unwrap();
        let top: Vec<_> = out.as_object().unwrap().keys().collect();
        assert_eq!(top, ["z", "a"]);
        let inner: Vec<_> = out["z"].as_object().unwrap().keys().collect();
        assert_eq!(inner, ["y", "x"]);
    }

    #[test]
    fn keeps_empty_string_values() {
        let out = unflatten(&m(json!({"a.b": ""})), ".").unwrap();
        assert_eq!(out, json!({"a": {"b": ""}}));
    }

    #[test]
    fn custom_separator() {
        let out = unflatten(&m(json!({"a/b": "x"})), "/").unwrap();
        assert_eq!(out, json!({"a": {"b": "x"}}));
    }

    #[test]
    fn empty_input_gives_empty_object() {
        assert_eq!(unflatten(&Map::new(), ".").unwrap(), json!({}));
    }

    #[test]
    fn leaf_then_parent_conflict() {
        let err = unflatten(&m(json!({"a": "x", "a.b": "y"})), ".").unwrap_err();
        assert_eq!(err, ConversionError::KeyConflict { key: "a.b".into() });
    }

    #[test]
    fn parent_then_leaf_conflict() {
        let err = unflatten(&m(json!({"a.b": "y", "a": "x"})), ".").unwrap_err();
        assert_eq!(err, ConversionError::KeyConflict { key: "a".into() });
    }

    #[test]
    fn rejects_numeric_segment() {
        let err = unflatten(&m(json!({"a.0.b": "x"})), ".").unwrap_err();
        assert_eq!(err, ConversionError::NumericKeySegment("0".into()));
    }

    #[test]
    fn rejects_empty_separator() {
        assert_eq!(
            unflatten(&m(json!({"a": "x"})), "").unwrap_err(),
            ConversionError::EmptySeparator
        );
    }

    #[test]
    fn round_trips_with_flatten() {
        let nested = json!({"user": {"name": "Name", "age": "Age"}, "ok": "OK"});
        let flat = crate::flatten(&nested, ".").unwrap();
        assert_eq!(unflatten(&flat, ".").unwrap(), nested);
    }
}
