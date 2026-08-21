use crate::auth::{Credentials, TokenProvider, provider_for};
use crate::error::SheetsError;
use crate::range::{a1_range, encode_path_segment, whole_tab_range};
use crate::{DEFAULT_BASE_URL, Table};
use reqwest::{Method, RequestBuilder, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

const USER_AGENT: &str = concat!("gslm/", env!("CARGO_PKG_VERSION"));

/// Builder for [`SheetsClient`]. Tests inject a base URL and/or a token provider.
pub struct SheetsClientBuilder {
    credentials: Credentials,
    base_url: String,
    provider: Option<Arc<dyn TokenProvider>>,
    connect_timeout: Duration,
    timeout: Duration,
}

/// Default TCP/TLS connect timeout.
pub const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Default whole-request timeout (connect + send + receive).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(60);

impl SheetsClientBuilder {
    pub fn new(credentials: Credentials) -> Self {
        Self {
            credentials,
            base_url: DEFAULT_BASE_URL.to_string(),
            provider: None,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Override the connect timeout (default 10s).
    pub fn connect_timeout(mut self, d: Duration) -> Self {
        self.connect_timeout = d;
        self
    }

    /// Override the whole-request timeout (default 60s). A timeout during the
    /// write step of [`SheetsClient::write_tab`] surfaces as
    /// [`SheetsError::WriteAfterClearFailed`].
    pub fn timeout(mut self, d: Duration) -> Self {
        self.timeout = d;
        self
    }

    /// Override the API origin (e.g. `http://127.0.0.1:1234`).
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into().trim_end_matches('/').to_string();
        self
    }

    /// Bypass credentials entirely with a custom token source.
    pub fn token_provider(mut self, provider: Arc<dyn TokenProvider>) -> Self {
        self.provider = Some(provider);
        self
    }

    pub async fn build(self) -> Result<SheetsClient, SheetsError> {
        let tls = tls_config()?;

        let provider = match self.provider {
            Some(p) => p,
            None => provider_for(self.credentials).await?,
        };
        let http = reqwest::Client::builder()
            .tls_backend_preconfigured(tls)
            .user_agent(USER_AGENT)
            .connect_timeout(self.connect_timeout)
            .timeout(self.timeout)
            .build()
            .map_err(|e| SheetsError::Network(e.to_string()))?;
        Ok(SheetsClient {
            http,
            provider,
            base_url: self.base_url,
        })
    }
}

/// Reads and writes whole Tabs. Cheap to clone (shares the HTTP pool and
/// token cache).
#[derive(Clone)]
pub struct SheetsClient {
    http: reqwest::Client,
    provider: Arc<dyn TokenProvider>,
    base_url: String,
}

#[derive(Deserialize)]
struct ValueRange {
    #[serde(default)]
    values: Vec<Vec<serde_json::Value>>,
}

#[derive(Serialize)]
struct WriteBody<'a> {
    range: &'a str,
    #[serde(rename = "majorDimension")]
    major_dimension: &'static str,
    values: &'a Table,
}

#[derive(Deserialize)]
struct GoogleErrorBody {
    error: GoogleErrorDetail,
}

#[derive(Deserialize)]
struct GoogleErrorDetail {
    #[serde(default)]
    message: String,
}

impl SheetsClient {
    pub fn builder(credentials: Credentials) -> SheetsClientBuilder {
        SheetsClientBuilder::new(credentials)
    }

    /// Read the whole tab as rows of strings. Trailing empty cells and rows
    /// are omitted by the API and therefore absent here too; an empty tab
    /// yields an empty table.
    pub async fn read_tab(&self, sheet_id: &str, tab: &str) -> Result<Table, SheetsError> {
        let url = format!(
            "{}/v4/spreadsheets/{}/values/{}?majorDimension=ROWS&valueRenderOption=FORMATTED_VALUE",
            self.base_url,
            encode_path_segment(sheet_id),
            encode_path_segment(&whole_tab_range(tab))
        );
        let resp = self
            .send(|| self.http.request(Method::GET, &url), sheet_id, tab)
            .await?;
        let body: ValueRange = resp
            .json()
            .await
            .map_err(|e| SheetsError::InvalidResponse(e.to_string()))?;
        Ok(body
            .values
            .into_iter()
            .map(|row| row.into_iter().map(cell_to_string).collect())
            .collect())
    }

    /// Replace the tab's content: clear everything, then write `table` from A1
    /// with `valueInputOption=RAW`. Not atomic; see
    /// [`SheetsError::WriteAfterClearFailed`].
    pub async fn write_tab(
        &self,
        sheet_id: &str,
        tab: &str,
        table: &Table,
    ) -> Result<(), SheetsError> {
        let clear_url = format!(
            "{}/v4/spreadsheets/{}/values/{}:clear",
            self.base_url,
            encode_path_segment(sheet_id),
            encode_path_segment(&whole_tab_range(tab))
        );
        self.send(
            || self.http.request(Method::POST, &clear_url),
            sheet_id,
            tab,
        )
        .await?;

        let range = a1_range(tab);
        let update_url = format!(
            "{}/v4/spreadsheets/{}/values/{}?valueInputOption=RAW",
            self.base_url,
            encode_path_segment(sheet_id),
            encode_path_segment(&range)
        );
        let body = WriteBody {
            range: &range,
            major_dimension: "ROWS",
            values: table,
        };
        self.send(
            || self.http.request(Method::PUT, &update_url).json(&body),
            sheet_id,
            tab,
        )
        .await
        .map(|_| ())
        .map_err(|e| SheetsError::WriteAfterClearFailed {
            source: Box::new(e),
        })
    }

    /// Send with a bearer token; on 401 invalidate the token and retry once.
    async fn send<F>(&self, make: F, sheet_id: &str, tab: &str) -> Result<Response, SheetsError>
    where
        F: Fn() -> RequestBuilder,
    {
        let mut retried = false;
        loop {
            let token = self.provider.token().await?;
            let resp = make()
                .bearer_auth(&token)
                .send()
                .await
                .map_err(|e| SheetsError::Network(e.to_string()))?;
            let status = resp.status();
            if status.is_success() {
                return Ok(resp);
            }
            if status == StatusCode::UNAUTHORIZED && !retried {
                retried = true;
                self.provider.invalidate().await;
                continue;
            }
            return Err(self.classify(status, resp, sheet_id, tab).await);
        }
    }

    async fn classify(
        &self,
        status: StatusCode,
        resp: Response,
        sheet_id: &str,
        tab: &str,
    ) -> SheetsError {
        let text = resp.text().await.unwrap_or_default();
        let message = serde_json::from_str::<GoogleErrorBody>(&text)
            .map(|b| b.error.message)
            .unwrap_or_else(|_| text.clone());
        match status.as_u16() {
            401 => SheetsError::Auth(message),
            403 => SheetsError::PermissionDenied {
                sheet_id: sheet_id.to_string(),
                service_account_email: self.provider.service_account_email(),
            },
            404 => SheetsError::SheetNotFound {
                sheet_id: sheet_id.to_string(),
            },
            400 if message.contains("Unable to parse range") => SheetsError::TabNotFound {
                sheet_id: sheet_id.to_string(),
                tab: tab.to_string(),
            },
            429 => SheetsError::RateLimited,
            s if (500..600).contains(&s) => SheetsError::ServerError { status: s },
            s => SheetsError::Http { status: s, message },
        }
    }
}

/// TLS config with the bundled Mozilla root store (`webpki-roots`), so the
/// client works in containers without `ca-certificates` (e.g. `node:*-slim`)
/// and behaves identically on every platform. Uses the crypto provider the
/// host process already installed (e.g. aws-lc-rs), falling back to ring.
fn tls_config() -> Result<rustls::ClientConfig, SheetsError> {
    let provider = rustls::crypto::CryptoProvider::get_default()
        .cloned()
        .unwrap_or_else(|| Arc::new(rustls::crypto::ring::default_provider()));
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let mut config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| SheetsError::Network(format!("TLS setup failed: {e}")))?
        .with_root_certificates(roots)
        .with_no_client_auth();
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
    Ok(config)
}

fn cell_to_string(v: serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s,
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}
