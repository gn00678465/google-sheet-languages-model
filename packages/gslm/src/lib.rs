#![deny(clippy::all)]

use napi::bindgen_prelude::*;
use napi_derive::napi;

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
    let separator = separator.as_deref().unwrap_or(gslm_core::DEFAULT_SEPARATOR);
    gslm_core::flatten(&value, separator).map_err(|e| Error::new(Status::InvalidArg, e.to_string()))
}

/// Version of the installed gslm package (read from package.json at build
/// time, so it always matches what `npm install` resolved).
#[napi]
pub fn version() -> String {
    env!("GSLM_PACKAGE_VERSION").to_string()
}
