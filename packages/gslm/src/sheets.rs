//! napi wrapper over `gslm_sheets::SheetsClient`.

use gslm_sheets::{Credentials, SheetsError};
use napi::bindgen_prelude::*;
use napi_derive::napi;

/// napi async functions cannot carry a custom `code`, so the code travels as
/// a `[CODE] ` prefix on the message; `index.js` lifts it onto `error.code`.
fn to_js(err: SheetsError) -> Error {
    Error::new(Status::GenericFailure, format!("[{}] {}", err.code(), err))
}

fn credentials_error(msg: &str) -> Error {
    Error::new(Status::GenericFailure, format!("[CREDENTIALS] {msg}"))
}

/// How to authenticate. Provide at most one field; omit all for Google
/// Application Default Credentials.
#[napi(object)]
#[derive(Default)]
pub struct CredentialsOptions {
    /// Path to a service-account JSON key file.
    pub file: Option<String>,
    /// Contents of a service-account JSON key.
    pub json: Option<String>,
    /// An already-obtained OAuth2 access token (never refreshed).
    pub access_token: Option<String>,
}

#[napi(object)]
#[derive(Default)]
pub struct SheetsClientOptions {
    pub credentials: Option<CredentialsOptions>,
    /// Override the Sheets API origin (tests / proxies).
    /// Default `https://sheets.googleapis.com`.
    pub base_url: Option<String>,
}

fn credentials_from(opts: Option<CredentialsOptions>) -> Result<Credentials> {
    let opts = opts.unwrap_or_default();
    let given = [
        opts.file.is_some(),
        opts.json.is_some(),
        opts.access_token.is_some(),
    ]
    .iter()
    .filter(|b| **b)
    .count();
    if given > 1 {
        return Err(credentials_error(
            "credentials: provide only one of file, json, accessToken",
        ));
    }
    Ok(if let Some(f) = opts.file {
        Credentials::ServiceAccountFile(f.into())
    } else if let Some(j) = opts.json {
        Credentials::ServiceAccountJson(j)
    } else if let Some(t) = opts.access_token {
        Credentials::AccessToken(t)
    } else {
        Credentials::ApplicationDefault
    })
}

fn credentials_from_target(target: &crate::config::ConfigTargetForClient) -> Result<Credentials> {
    match target.credentials.kind.as_str() {
        "file" => target
            .credentials
            .path
            .as_ref()
            .map(|path| Credentials::ServiceAccountFile(path.into()))
            .ok_or_else(|| credentials_error("config Target 的 file credentials 缺少 path")),
        "json" => {
            let env_name =
                target.credentials.env.as_ref().ok_or_else(|| {
                    credentials_error("config Target 的 json credentials 缺少 env")
                })?;
            let from_env = std::env::var(env_name)
                .ok()
                .filter(|value| !value.is_empty());
            let json = match from_env {
                Some(value) => Some(value),
                None => dotenv_value(target.dotenv_path.as_deref(), env_name)?,
            }
            .ok_or_else(|| credentials_error(&format!("缺少或為空的環境變數 {env_name}")))?;
            Ok(Credentials::ServiceAccountJson(json))
        }
        "adc" => Ok(Credentials::ApplicationDefault),
        kind => Err(credentials_error(&format!(
            "config Target 的 credentials.kind 不支援 `{kind}`"
        ))),
    }
}

fn dotenv_value(path: Option<&str>, env_name: &str) -> Result<Option<String>> {
    let Some(path) = path.map(std::path::Path::new).filter(|path| path.is_file()) else {
        return Ok(None);
    };
    let values = dotenvy::from_path_iter(path)
        .map_err(|error| credentials_error(&format!("無法讀取 {}: {error}", path.display())))?;
    for value in values {
        let (name, value) = value
            .map_err(|error| credentials_error(&format!("無法解析 {}: {error}", path.display())))?;
        if name == env_name && !value.is_empty() {
            return Ok(Some(value));
        }
    }
    Ok(None)
}

async fn build_client(credentials: Credentials, base_url: Option<String>) -> Result<SheetsClient> {
    let mut builder = gslm_sheets::SheetsClient::builder(credentials);
    if let Some(url) = base_url {
        builder = builder.base_url(url);
    }
    let inner = builder.build().await.map_err(to_js)?;
    Ok(SheetsClient { inner })
}

/// Reads and writes whole Tabs of a Google Sheet.
#[napi]
pub struct SheetsClient {
    inner: gslm_sheets::SheetsClient,
}

#[napi]
impl SheetsClient {
    /// Create a client. Credentials are validated here (file readable,
    /// JSON is a service-account key); no token is exchanged yet.
    #[napi(factory)]
    pub async fn create(options: Option<SheetsClientOptions>) -> Result<SheetsClient> {
        let options = options.unwrap_or_default();
        let creds = credentials_from(options.credentials)?;
        build_client(creds, options.base_url).await
    }

    /// Create a client from a Target returned by `loadConfig`. JSON
    /// credentials are read from their named environment variable here, so no
    /// secret crosses the JavaScript boundary.
    #[napi(factory)]
    pub async fn from_config(target: crate::config::ConfigTargetForClient) -> Result<SheetsClient> {
        build_client(credentials_from_target(&target)?, None).await
    }

    /// Read the whole tab as rows of strings (header row first). Feed the
    /// result to `sheetToModel`.
    #[napi]
    pub async fn read_tab(&self, sheet_id: String, tab: String) -> Result<Vec<Vec<String>>> {
        self.inner.read_tab(&sheet_id, &tab).await.map_err(to_js)
    }

    /// Replace the tab's content with `rows` (clear, then write from A1 with
    /// RAW input). Use `modelToSheet` to build `rows`.
    #[napi]
    pub async fn write_tab(
        &self,
        sheet_id: String,
        tab: String,
        rows: Vec<Vec<String>>,
    ) -> Result<()> {
        self.inner
            .write_tab(&sheet_id, &tab, &rows)
            .await
            .map_err(to_js)
    }
}
