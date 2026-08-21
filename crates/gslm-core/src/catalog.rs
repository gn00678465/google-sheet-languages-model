use crate::error::{ConversionError, json_type_name};
use crate::flatten::flatten;
use crate::unflatten::unflatten;
use indexmap::IndexMap;
use serde_json::{Map, Value};

/// On-disk structure of a Catalog. Only affects output; input is auto-detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Nest,
    Flat,
}

/// All translations of a single locale. Stored flat internally
/// (`key → translation`), insertion-ordered. Translations are always strings;
/// an empty string is a legitimate translation and is preserved.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Catalog {
    entries: IndexMap<String, String>,
}

impl Catalog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build from a JSON object in nest, flat, or mixed shape.
    /// Every leaf must be a string.
    pub fn from_value(value: &Value, separator: &str) -> Result<Self, ConversionError> {
        let flat = flatten(value, separator)?;
        let mut entries = IndexMap::with_capacity(flat.len());
        for (key, leaf) in flat {
            match leaf {
                Value::String(s) => {
                    entries.insert(key, s);
                }
                other => {
                    return Err(ConversionError::NonStringTranslation {
                        key,
                        kind: json_type_name(&other),
                    });
                }
            }
        }
        Ok(Self { entries })
    }

    pub fn from_entries<I, K, V>(entries: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        Self {
            entries: entries
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries.get(key).map(String::as_str)
    }

    pub fn insert(&mut self, key: impl Into<String>, translation: impl Into<String>) {
        self.entries.insert(key.into(), translation.into());
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.entries.contains_key(key)
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.entries.keys().map(String::as_str)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &IndexMap<String, String> {
        &self.entries
    }

    /// Serialize in the requested format. Nest may fail on numeric segments or
    /// key conflicts; Flat never fails.
    pub fn to_value(&self, format: Format, separator: &str) -> Result<Value, ConversionError> {
        let flat: Map<String, Value> = self
            .entries
            .iter()
            .map(|(k, v)| (k.clone(), Value::String(v.clone())))
            .collect();
        match format {
            Format::Flat => Ok(Value::Object(flat)),
            Format::Nest => unflatten(&flat, separator),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn keys(c: &Catalog) -> Vec<&str> {
        c.keys().collect()
    }

    #[test]
    fn from_nest() {
        let c = Catalog::from_value(&json!({"user": {"name": "Name"}, "ok": "OK"}), ".").unwrap();
        assert_eq!(keys(&c), ["user.name", "ok"]);
        assert_eq!(c.get("user.name"), Some("Name"));
    }

    #[test]
    fn from_flat() {
        let c = Catalog::from_value(&json!({"user.name": "Name", "ok": "OK"}), ".").unwrap();
        assert_eq!(keys(&c), ["user.name", "ok"]);
    }

    #[test]
    fn from_mixed_shape() {
        let c = Catalog::from_value(&json!({"user": {"name": "N"}, "user.age": "A"}), ".").unwrap();
        assert_eq!(keys(&c), ["user.name", "user.age"]);
    }

    #[test]
    fn keeps_empty_string() {
        let c = Catalog::from_value(&json!({"a": ""}), ".").unwrap();
        assert_eq!(c.get("a"), Some(""));
    }

    #[test]
    fn rejects_non_string_leaves_with_key() {
        for (v, kind) in [
            (json!({"a": {"n": 1}}), "number"),
            (json!({"a": {"n": true}}), "boolean"),
            (json!({"a": {"n": null}}), "null"),
        ] {
            assert_eq!(
                Catalog::from_value(&v, ".").unwrap_err(),
                ConversionError::NonStringTranslation {
                    key: "a.n".into(),
                    kind
                }
            );
        }
    }

    #[test]
    fn rejects_arrays_and_numeric_segments() {
        assert_eq!(
            Catalog::from_value(&json!({"days": ["Mon"]}), ".").unwrap_err(),
            ConversionError::ArrayNotSupported("days".into())
        );
        assert_eq!(
            Catalog::from_value(&json!({"0": "x"}), ".").unwrap_err(),
            ConversionError::NumericKeySegment("0".into())
        );
    }

    #[test]
    fn to_flat_and_nest() {
        let c = Catalog::from_entries([("user.name", "Name"), ("ok", "")]);
        assert_eq!(
            c.to_value(Format::Flat, ".").unwrap(),
            json!({"user.name": "Name", "ok": ""})
        );
        assert_eq!(
            c.to_value(Format::Nest, ".").unwrap(),
            json!({"user": {"name": "Name"}, "ok": ""})
        );
    }

    #[test]
    fn to_nest_reports_conflict() {
        let c = Catalog::from_entries([("a", "x"), ("a.b", "y")]);
        assert_eq!(
            c.to_value(Format::Nest, ".").unwrap_err(),
            ConversionError::KeyConflict { key: "a.b".into() }
        );
    }

    #[test]
    fn round_trip_nest() {
        let nested = json!({"user": {"name": "Name", "age": ""}, "ok": "OK"});
        let c = Catalog::from_value(&nested, ".").unwrap();
        assert_eq!(c.to_value(Format::Nest, ".").unwrap(), nested);
    }
}
