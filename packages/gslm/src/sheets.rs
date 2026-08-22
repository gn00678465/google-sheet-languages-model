//! napi wrapper over `gslm_sheets::SheetsClient`.

use gslm_config::CredentialsSource;
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
    let source = crate::config::credentials_for_handle(&target.credential_handle)
        .ok_or_else(|| credentials_error("config Target 的 credentialHandle 無效或已過期"))?;
    match source {
        CredentialsSource::File(path) => Ok(Credentials::ServiceAccountFile(path)),
        CredentialsSource::Json { value, .. } => Ok(Credentials::ServiceAccountJson(value)),
        CredentialsSource::ApplicationDefault => Ok(Credentials::ApplicationDefault),
    }
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

    /// Create a client from a Target returned by `loadConfig`. The
    /// process-local opaque handle keeps credential JSON inside Rust.
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
