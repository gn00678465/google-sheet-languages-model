#![deny(clippy::all)]

//! Thin napi-rs wrapper over `gslm-core`. No logic lives here: every function
//! converts JS values to core types, calls core, and maps errors to JS `Error`.

pub mod config;
mod sheets;

use gslm_core::{Catalog, ConversionError, DEFAULT_SEPARATOR, Model};
use indexmap::IndexMap;
use napi::bindgen_prelude::*;
use napi_derive::napi;

fn to_js(err: ConversionError) -> Error {
    Error::new(Status::InvalidArg, err.to_string())
}

fn sep(separator: &Option<String>) -> &str {
    separator.as_deref().unwrap_or(DEFAULT_SEPARATOR)
}

/// Flatten a nested catalog object into a single-level object whose keys are
/// joined by `separator` (default `"."`). Key order is preserved.
///
/// Throws if the input is not a plain object, if any key segment is a number,
/// or if `separator` is empty.
#[napi]
pub fn flatten(
    value: serde_json::Value,
    separator: Option<String>,
) -> Result<serde_json::Map<String, serde_json::Value>> {
    gslm_core::flatten(&value, sep(&separator)).map_err(to_js)
}

/// Inverse of `flatten`: rebuild a nested object from dotted keys.
/// Key order is preserved at every level.
///
/// Throws on numeric key segments, on conflicts such as `a` together with
/// `a.b`, or if `separator` is empty.
#[napi]
pub fn unflatten(
    flat: serde_json::Map<String, serde_json::Value>,
    separator: Option<String>,
) -> Result<serde_json::Value> {
    gslm_core::unflatten(&flat, sep(&separator)).map_err(to_js)
}

/// A Model as plain data: an ordered list of locales (the first is the Source
/// locale) and one flat catalog per locale. Serializable as JSON.
#[napi(object, js_name = "Model")]
pub struct JsModel {
    pub locales: Vec<String>,
    /// `locale → (key → translation)`. Missing keys mean "not translated";
    /// empty strings are kept as-is.
    pub catalogs: IndexMap<String, IndexMap<String, String>>,
}

impl From<Model> for JsModel {
    fn from(model: Model) -> Self {
        let catalogs = model
            .catalogs()
            .iter()
            .map(|(locale, catalog)| (locale.clone(), catalog.entries().clone()))
            .collect();
        JsModel {
            locales: model.locales().to_vec(),
            catalogs,
        }
    }
}

impl TryFrom<JsModel> for Model {
    type Error = Error;

    fn try_from(js: JsModel) -> Result<Model> {
        let mut model = Model::new(js.locales);
        for (locale, entries) in js.catalogs {
            model
                .set_catalog(&locale, Catalog::from_entries(entries))
                .map_err(to_js)?;
        }
        Ok(model)
    }
}

/// Parse Tab rows (first row = header: key column + locale columns) into a
/// Model for the requested locales. Columns are matched by header text; the
/// first column is always the key column. Empty cells are missing
/// translations; rows with an empty key are skipped.
///
/// Throws if the sheet is empty, a requested locale is not in the header, or a
/// key appears twice.
#[napi]
pub fn sheet_to_model(rows: Vec<Vec<String>>, locales: Vec<String>) -> Result<JsModel> {
    Model::from_table(&rows, locales)
        .map(JsModel::from)
        .map_err(to_js)
}

/// Build Tab rows from a Model: header `["key", ...locales]`, then one row
/// per key in Source-locale order followed by keys that exist only in other
/// locales. Missing translations and empty strings both become `""`.
#[napi]
pub fn model_to_sheet(model: JsModel) -> Result<Vec<Vec<String>>> {
    Model::try_from(model).map(|m| m.to_table())
}

/// Keys that exist in some non-Source locale but not in the Source locale,
/// in the order `modelToSheet` appends them.
#[napi]
pub fn orphan_keys(model: JsModel) -> Result<Vec<String>> {
    Model::try_from(model).map(|m| m.orphan_keys())
}

/// Version of the installed gslm package (read from package.json at build
/// time, so it always matches what `npm install` resolved).
#[napi]
pub fn version() -> String {
    env!("GSLM_PACKAGE_VERSION").to_string()
}
