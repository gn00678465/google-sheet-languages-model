//! napi data adapters for `gslm-config`.

use gslm_config::{
    ConfigError, CredentialsSource, LoadOptions, Overrides, ResolvedConfig, ResolvedTarget,
};
use gslm_core::Format;
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

static CREDENTIALS_BY_HANDLE: LazyLock<Mutex<HashMap<CredentialHandle, CredentialsSource>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CredentialHandle(String);

impl CredentialHandle {
    fn generate() -> Result<Self> {
        let mut bytes = [0_u8; 16];
        getrandom::fill(&mut bytes).map_err(|_| {
            Error::new(
                Status::GenericFailure,
                "[CONFIG_INVALID] 無法建立憑證 handle",
            )
        })?;
        Ok(Self(format!(
            "gslm-credential-{}",
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        )))
    }
}

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
    /// Opaque, process-local handle used by [`SheetsClient::from_config`]. It
    /// contains no credential data, but must be retained with this Target.
    pub credential_handle: String,
}

/// Internal input for `SheetsClient.fromConfig`. It deliberately contains
/// only the opaque credential handle returned by `loadConfig`.
#[napi(object)]
#[derive(Debug, Clone)]
pub struct ConfigTargetForClient {
    pub credential_handle: String,
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
        let details = gslm_cli::credential_details(&value);
        Self {
            kind: details.kind.into(),
            path: details.path.map(|path| path.to_string_lossy().into_owned()),
            env: details.env_name,
        }
    }
}

impl TryFrom<ResolvedTarget> for ConfigTarget {
    type Error = Error;

    fn try_from(value: ResolvedTarget) -> Result<Self> {
        let credential_handle = register_credentials(&value.credentials)?;
        Ok(Self {
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
            credential_handle: credential_handle.0,
        })
    }
}

fn register_credentials(credentials: &CredentialsSource) -> Result<CredentialHandle> {
    let handle = CredentialHandle::generate()?;
    CREDENTIALS_BY_HANDLE
        .lock()
        .expect("credential registry lock must not be poisoned")
        .insert(handle.clone(), credentials.clone());
    Ok(handle)
}

/// Resolve the process-local credential source attached to a Config Target.
/// The returned value is never serialized across the N-API boundary.
pub(crate) fn credentials_for_handle(handle: &str) -> Option<CredentialsSource> {
    CREDENTIALS_BY_HANDLE
        .lock()
        .ok()
        .and_then(|credentials| credentials.get(&CredentialHandle(handle.into())).cloned())
}

/// Rehydrate a safe JS Target into its Rust-only credential-bearing form.
/// This is deliberately process-local: calling it with a copied JSON value
/// fails rather than falling back to parsing environment variables again.
pub(crate) fn resolved_target(target: ConfigTarget) -> Result<ResolvedTarget> {
    let credentials = credentials_for_handle(&target.credential_handle).ok_or_else(|| {
        Error::new(
            Status::InvalidArg,
            "[CREDENTIALS] Target 不含可用的憑證 handle；請直接使用 loadConfig 回傳的 Target",
        )
    })?;
    let format = match target.format.as_str() {
        "nest" => Format::Nest,
        "flat" => Format::Flat,
        _ => {
            return Err(Error::new(
                Status::InvalidArg,
                "[CONFIG_INVALID] Target 的 format 必須是 nest 或 flat",
            ));
        }
    };
    Ok(ResolvedTarget {
        name: target.name,
        sheet: target.sheet,
        tab: target.tab,
        locales: target.locales,
        path: PathBuf::from(target.path),
        format,
        key_separator: target.key_separator,
        credentials,
    })
}

/// Drop credentials once JavaScript no longer retains the Target that owns
/// this opaque handle. Unknown handles are intentionally harmless.
#[napi]
pub fn release_config_credentials(handle: String) {
    if let Ok(mut credentials) = CREDENTIALS_BY_HANDLE.lock() {
        credentials.remove(&CredentialHandle(handle));
    }
}

impl TryFrom<ResolvedConfig> for JsResolvedConfig {
    type Error = Error;

    fn try_from(value: ResolvedConfig) -> Result<Self> {
        Ok(Self {
            config_path: value
                .config_path
                .map(|path| path.to_string_lossy().into_owned()),
            targets: value
                .targets
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<_>>>()?,
            warnings: value.warnings,
        })
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
    gslm_config::load(native).map_err(to_js)?.try_into()
}

/// JSON Schema draft 2020-12 generated from the Rust raw-config types.
#[napi]
pub fn config_schema() -> serde_json::Value {
    gslm_config::schema()
}
