//! napi data adapters for `gslm-config`.

use gslm_config::{
    ConfigError, CredentialsSource, LoadOptions, Overrides, ResolvedConfig, ResolvedTarget,
};
use gslm_core::Format;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::BTreeMap;
use std::path::PathBuf;

fn to_js(error: ConfigError) -> Error {
    Error::new(Status::InvalidArg, format!("[{}] {}", error.code(), error))
}

/// Options accepted by [`load_config`]. `env` is mainly useful for tests and
/// embedded callers that need an isolated process environment.
#[napi(object)]
#[derive(Default)]
pub struct LoadConfigOptions {
    pub cwd: Option<String>,
    pub config_path: Option<String>,
    pub env: Option<BTreeMap<String, String>>,
    pub overrides: Option<ConfigOverrides>,
    pub targets: Option<Vec<String>>,
    pub load_dotenv: Option<bool>,
}

/// CLI-style overrides with the same precedence and validation as `GSLM_*`.
#[napi(object)]
#[derive(Default)]
pub struct ConfigOverrides {
    pub sheet: Option<String>,
    pub tab: Option<String>,
    pub locales: Option<Vec<String>>,
    pub path: Option<String>,
    pub format: Option<String>,
    pub key_separator: Option<String>,
    pub credentials: Option<String>,
    pub credentials_json: Option<String>,
}

impl TryFrom<ConfigOverrides> for Overrides {
    type Error = ConfigError;

    fn try_from(value: ConfigOverrides) -> std::result::Result<Self, Self::Error> {
        let format = value
            .format
            .map(|format| match format.as_str() {
                "nest" => Ok(Format::Nest),
                "flat" => Ok(Format::Flat),
                _ => Err(ConfigError::Invalid {
                    path: None,
                    field: "overrides.format".into(),
                    message: "只能是 `nest` 或 `flat`".into(),
                }),
            })
            .transpose()?;
        Ok(Overrides {
            sheet: value.sheet,
            tab: value.tab,
            locales: value.locales,
            path: value.path,
            format,
            key_separator: value.key_separator,
            credentials: value.credentials,
            credentials_json: value.credentials_json,
        })
    }
}

/// Safe credential metadata. It never contains credential JSON.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct ConfigCredentials {
    pub kind: String,
    pub path: Option<String>,
    pub env: Option<String>,
}

/// A Target ready for pull or push. `path` is an absolute template.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct ConfigTarget {
    pub name: String,
    pub sheet: String,
    pub tab: String,
    pub locales: Vec<String>,
    pub path: String,
    pub format: String,
    pub key_separator: String,
    pub credentials: ConfigCredentials,
}

/// Fully resolved config data that is safe to serialize or print.
#[napi(object)]
pub struct JsResolvedConfig {
    pub config_path: Option<String>,
    pub targets: Vec<ConfigTarget>,
    pub warnings: Vec<String>,
}

impl From<CredentialsSource> for ConfigCredentials {
    fn from(value: CredentialsSource) -> Self {
        match value {
            CredentialsSource::File(path) => Self {
                kind: "file".into(),
                path: Some(path.to_string_lossy().into_owned()),
                env: None,
            },
            CredentialsSource::Json { env_name, .. } => Self {
                kind: "json".into(),
                path: None,
                env: Some(env_name),
            },
            CredentialsSource::ApplicationDefault => Self {
                kind: "adc".into(),
                path: None,
                env: None,
            },
        }
    }
}

impl From<ResolvedTarget> for ConfigTarget {
    fn from(value: ResolvedTarget) -> Self {
        Self {
            name: value.name,
            sheet: value.sheet,
            tab: value.tab,
            locales: value.locales,
            path: value.path.to_string_lossy().into_owned(),
            format: match value.format {
                Format::Nest => "nest".into(),
                Format::Flat => "flat".into(),
            },
            key_separator: value.key_separator,
            credentials: value.credentials.into(),
        }
    }
}

impl From<ResolvedConfig> for JsResolvedConfig {
    fn from(value: ResolvedConfig) -> Self {
        Self {
            config_path: value
                .config_path
                .map(|path| path.to_string_lossy().into_owned()),
            targets: value.targets.into_iter().map(Into::into).collect(),
            warnings: value.warnings,
        }
    }
}

/// Discover and load the config synchronously. File I/O is intentionally
/// small, and all errors carry a stable `[CONFIG_*]` prefix for `index.js`.
#[napi]
pub fn load_config(options: Option<LoadConfigOptions>) -> Result<JsResolvedConfig> {
    let options = options.unwrap_or_default();
    let mut native = LoadOptions::default();
    if let Some(cwd) = options.cwd {
        native.cwd = PathBuf::from(cwd);
    }
    native.config_path = options.config_path.map(PathBuf::from);
    if let Some(env) = options.env {
        native.env = env;
    }
    native.overrides = options
        .overrides
        .map(TryInto::try_into)
        .transpose()
        .map_err(to_js)?
        .unwrap_or_default();
    native.targets = options.targets;
    native.load_dotenv = options.load_dotenv.unwrap_or(true);
    gslm_config::load(native).map(Into::into).map_err(to_js)
}

/// JSON Schema draft 2020-12 generated from the Rust raw-config types.
#[napi]
pub fn config_schema() -> serde_json::Value {
    gslm_config::schema()
}
