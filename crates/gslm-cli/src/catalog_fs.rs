use crate::CliError;
use gslm_core::{Catalog, Format};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// Observed top-level shape of a JSON Catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Nested,
    Flat,
    Empty,
}

/// Catalog plus the source form facts needed for configuration-drift checks.
#[derive(Debug, Clone)]
pub(crate) struct ReadCatalog {
    pub catalog: Catalog,
    pub shape: Shape,
    pub flat_key_has_separator: bool,
}

/// Substitute every `{locale}` placeholder in an already-resolved template.
pub fn render_path(template: &Path, locale: &str) -> PathBuf {
    PathBuf::from(template.to_string_lossy().replace("{locale}", locale))
}

/// Classify a parsed JSON Catalog without attempting conversion.
pub fn detect_shape(value: &Value) -> Shape {
    let Some(object) = value.as_object() else {
        return Shape::Flat;
    };
    if object.is_empty() {
        Shape::Empty
    } else if object.values().any(Value::is_object) {
        Shape::Nested
    } else {
        Shape::Flat
    }
}

pub(crate) fn read_catalog(path: &Path, separator: &str) -> Result<Option<ReadCatalog>, CliError> {
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => {
            return Err(CliError::Catalog {
                path: path.to_path_buf(),
                reason: "無法讀取檔案".into(),
            });
        }
    };
    let value: Value = serde_json::from_str(&raw).map_err(|_| CliError::Catalog {
        path: path.to_path_buf(),
        reason: "JSON 格式無效".into(),
    })?;
    let flat_key_has_separator = value
        .as_object()
        .is_some_and(|object| object.keys().any(|key| key.contains(separator)));
    let shape = detect_shape(&value);
    let catalog = Catalog::from_value(&value, separator).map_err(|error| CliError::Catalog {
        path: path.to_path_buf(),
        reason: crate::core_message(&error),
    })?;
    Ok(Some(ReadCatalog {
        catalog,
        shape,
        flat_key_has_separator,
    }))
}

/// Result of a pull write, classified by content rather than timestamps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteOutcome {
    Created,
    Updated,
    Unchanged,
}

impl WriteOutcome {
    /// Stable lowercase spelling returned by the JavaScript API.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::Unchanged => "unchanged",
        }
    }
}

pub(crate) fn write_catalog(
    path: &Path,
    catalog: &Catalog,
    format: Format,
    separator: &str,
) -> Result<WriteOutcome, CliError> {
    let value = catalog.to_value(format, separator)?;
    let rendered = format!(
        "{}\n",
        serde_json::to_string_pretty(&value).expect("JSON value serializes")
    );
    match fs::read(path) {
        Ok(existing) if existing == rendered.as_bytes() => return Ok(WriteOutcome::Unchanged),
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(CliError::Io {
                path: path.to_path_buf(),
                source: error,
            });
        }
    }
    let outcome = if path.exists() {
        WriteOutcome::Updated
    } else {
        WriteOutcome::Created
    };
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|source| CliError::Io {
        path: parent.to_path_buf(),
        source,
    })?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|source| CliError::Io {
        path: parent.to_path_buf(),
        source,
    })?;
    restrict_temporary_permissions(&temporary, path)?;
    use std::io::Write;
    temporary
        .write_all(rendered.as_bytes())
        .and_then(|_| temporary.flush())
        .map_err(|source| CliError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    temporary.persist(path).map_err(|error| CliError::Io {
        path: path.to_path_buf(),
        source: error.error,
    })?;
    Ok(outcome)
}

#[cfg(unix)]
fn restrict_temporary_permissions(
    temporary: &tempfile::NamedTempFile,
    destination: &Path,
) -> Result<(), CliError> {
    use std::os::unix::fs::PermissionsExt;

    let permissions = match fs::metadata(destination) {
        Ok(metadata) => metadata.permissions(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::Permissions::from_mode(0o644)
        }
        Err(source) => {
            return Err(CliError::Io {
                path: destination.to_path_buf(),
                source,
            });
        }
    };
    temporary
        .as_file()
        .set_permissions(permissions)
        .map_err(|source| CliError::Io {
            path: destination.to_path_buf(),
            source,
        })
}

#[cfg(not(unix))]
fn restrict_temporary_permissions(
    _temporary: &tempfile::NamedTempFile,
    _destination: &Path,
) -> Result<(), CliError> {
    Ok(())
}

pub(crate) fn shape_mismatch(read: &ReadCatalog, format: Format) -> bool {
    match (format, read.shape) {
        (Format::Flat, Shape::Nested) => true,
        (Format::Nest, Shape::Flat) => read.flat_key_has_separator,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::tempdir;

    #[test]
    fn shape_detects_empty_flat_and_nested() {
        assert_eq!(detect_shape(&json!({})), Shape::Empty);
        assert_eq!(detect_shape(&json!({"title": "Title"})), Shape::Flat);
        assert_eq!(
            detect_shape(&json!({"user": {"name": "Name"}})),
            Shape::Nested
        );
    }

    #[test]
    fn path_replaces_every_locale_marker() {
        assert_eq!(
            render_path(Path::new("locales/{locale}/{locale}.json"), "zh-TW"),
            PathBuf::from("locales/zh-TW/zh-TW.json")
        );
    }

    #[test]
    fn write_outcomes_have_stable_api_spellings() {
        assert_eq!(WriteOutcome::Created.as_str(), "created");
        assert_eq!(WriteOutcome::Updated.as_str(), "updated");
        assert_eq!(WriteOutcome::Unchanged.as_str(), "unchanged");
    }

    #[test]
    fn catalog_reader_classifies_missing_invalid_and_unreadable_inputs() {
        let directory = tempdir().unwrap();
        let missing = directory.path().join("missing.json");
        assert!(read_catalog(&missing, ".").unwrap().is_none());

        let invalid = directory.path().join("invalid.json");
        fs::write(&invalid, "not json").unwrap();
        let error = read_catalog(&invalid, ".").unwrap_err();
        assert!(matches!(
            error,
            CliError::Catalog { ref path, ref reason }
                if path == &invalid && reason == "JSON 格式無效"
        ));

        let array = directory.path().join("array.json");
        fs::write(&array, r#"{"items":["not supported"]}"#).unwrap();
        let error = read_catalog(&array, ".").unwrap_err();
        assert!(matches!(
            error,
            CliError::Catalog { ref path, ref reason }
                if path == &array && reason.contains("Catalog 不支援陣列（key：items）")
        ));

        let unreadable = directory.path().join("directory.json");
        fs::create_dir(&unreadable).unwrap();
        let error = read_catalog(&unreadable, ".").unwrap_err();
        assert!(matches!(
            error,
            CliError::Catalog { ref path, ref reason }
                if path == &unreadable && reason == "無法讀取檔案"
        ));
    }

    #[test]
    fn catalog_writer_reports_content_outcomes_and_parent_io_failures() {
        let directory = tempdir().unwrap();
        let output = directory.path().join("nested/locales/en.json");
        let mut catalog = Catalog::from_entries([("title", "Title")]);

        assert_eq!(
            write_catalog(&output, &catalog, Format::Nest, ".").unwrap(),
            WriteOutcome::Created
        );
        assert_eq!(
            fs::read_to_string(&output).unwrap(),
            "{\n  \"title\": \"Title\"\n}\n"
        );
        assert_eq!(
            write_catalog(&output, &catalog, Format::Nest, ".").unwrap(),
            WriteOutcome::Unchanged
        );
        catalog.insert("body", "Body");
        assert_eq!(
            write_catalog(&output, &catalog, Format::Flat, ".").unwrap(),
            WriteOutcome::Updated
        );
        assert_eq!(
            fs::read_to_string(&output).unwrap(),
            "{\n  \"title\": \"Title\",\n  \"body\": \"Body\"\n}\n"
        );

        let blocking_parent = directory.path().join("not-a-directory");
        fs::write(&blocking_parent, "file").unwrap();
        let blocked_output = blocking_parent.join("en.json");
        let error = write_catalog(&blocked_output, &catalog, Format::Flat, ".").unwrap_err();
        assert!(matches!(
            error,
            CliError::Io { ref path, .. } if path == &blocked_output
        ));
    }

    #[test]
    fn format_drift_requires_a_real_shape_mismatch() {
        let nested = ReadCatalog {
            catalog: Catalog::default(),
            shape: Shape::Nested,
            flat_key_has_separator: false,
        };
        let flat_with_separator = ReadCatalog {
            catalog: Catalog::default(),
            shape: Shape::Flat,
            flat_key_has_separator: true,
        };
        let flat_without_separator = ReadCatalog {
            catalog: Catalog::default(),
            shape: Shape::Flat,
            flat_key_has_separator: false,
        };

        assert!(shape_mismatch(&nested, Format::Flat));
        assert!(shape_mismatch(&flat_with_separator, Format::Nest));
        assert!(!shape_mismatch(&flat_without_separator, Format::Nest));
        assert!(!shape_mismatch(&nested, Format::Nest));
    }
}
