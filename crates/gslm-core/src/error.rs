use thiserror::Error;

/// Every way a conversion between Catalog / Model / Tab can fail.
/// Messages include the offending key (and row where applicable) so users can
/// locate the problem among thousands of translations.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ConversionError {
    #[error("expected an object at the top level, got {0}")]
    NotAnObject(&'static str),
    #[error("key segment must not be a number: {0:?}")]
    NumericKeySegment(String),
    #[error("separator must not be empty")]
    EmptySeparator,
    #[error("arrays are not supported in catalogs (at key {0:?})")]
    ArrayNotSupported(String),
    #[error("translation must be a string, got {kind} (at key {key:?})")]
    NonStringTranslation { key: String, kind: &'static str },
    #[error("key {key:?} conflicts with a nested key under the same prefix")]
    KeyConflict { key: String },
    #[error(
        "key {key:?} appears more than once after flattening (nested and dotted forms both present)"
    )]
    DuplicateFlatKey { key: String },
    #[error("sheet is empty (no header row); check the tab name")]
    EmptySheet,
    #[error("locale {locale:?} not found in header row; available columns: {available:?}")]
    LocaleNotInHeader {
        locale: String,
        available: Vec<String>,
    },
    #[error("duplicate key {key:?} at row {row} (1-based, header is row 1)")]
    DuplicateKey { key: String, row: usize },
    #[error("locale {0:?} is not part of this model")]
    UnknownLocale(String),
}

pub(crate) fn json_type_name(value: &serde_json::Value) -> &'static str {
    use serde_json::Value::*;
    match value {
        Null => "null",
        Bool(_) => "boolean",
        Number(_) => "number",
        String(_) => "string",
        Array(_) => "array",
        Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::json_type_name;
    use serde_json::json;

    #[test]
    fn json_type_names_match_json_values_used_in_validation_errors() {
        assert_eq!(json_type_name(&json!(null)), "null");
        assert_eq!(json_type_name(&json!(true)), "boolean");
        assert_eq!(json_type_name(&json!(1)), "number");
        assert_eq!(json_type_name(&json!("text")), "string");
        assert_eq!(json_type_name(&json!([])), "array");
        assert_eq!(json_type_name(&json!({})), "object");
    }
}
