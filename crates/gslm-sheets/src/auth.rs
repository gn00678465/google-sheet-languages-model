//! Token acquisition. `gcp_auth` handles service accounts and Application
//! Default Credentials; a static token covers pre-obtained OAuth tokens and
//! tests.

use crate::SCOPE;
use crate::error::SheetsError;
use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Where the access token comes from.
#[derive(Debug, Clone)]
pub enum Credentials {
    /// Path to a service-account JSON key file.
    ServiceAccountFile(PathBuf),
    /// Contents of a service-account JSON key.
    ServiceAccountJson(String),
    /// Google Application Default Credentials
    /// (`GOOGLE_APPLICATION_CREDENTIALS` → gcloud ADC file → metadata server).
    ApplicationDefault,
    /// An already-obtained OAuth2 access token. Never refreshed.
    AccessToken(String),
}

/// Source of bearer tokens. Implemented by `gcp_auth` and by static tokens;
/// injectable for tests.
#[async_trait]
pub trait TokenProvider: Send + Sync + std::fmt::Debug {
    /// A currently valid access token for the Sheets scope.
    async fn token(&self) -> Result<String, SheetsError>;
    /// Drop any cached token so the next call fetches a fresh one.
    async fn invalidate(&self);
    /// The service-account email, when known (used in permission errors).
    fn service_account_email(&self) -> Option<String> {
        None
    }
}

/// A fixed token.
pub struct StaticToken(pub String);

impl std::fmt::Debug for StaticToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StaticToken(<redacted>)")
    }
}

#[async_trait]
impl TokenProvider for StaticToken {
    async fn token(&self) -> Result<String, SheetsError> {
        Ok(self.0.clone())
    }
    async fn invalidate(&self) {}
}

/// How to (re)build the underlying `gcp_auth` provider. Kept so that
/// `invalidate` can drop the cached token by constructing a fresh provider.
enum GcpSource {
    ServiceAccountJson(String),
    ApplicationDefault,
}

impl GcpSource {
    async fn provider(&self) -> Result<Arc<dyn gcp_auth::TokenProvider>, SheetsError> {
        match self {
            GcpSource::ServiceAccountJson(raw) => {
                let sa = gcp_auth::CustomServiceAccount::from_json(raw)
                    .map_err(|e| SheetsError::Credentials(e.to_string()))?;
                Ok(Arc::new(sa))
            }
            GcpSource::ApplicationDefault => gcp_auth::provider().await.map_err(|e| {
                SheetsError::Credentials(format!(
                    "application default credentials unavailable: {e}"
                ))
            }),
        }
    }
}

struct GcpAuth {
    source: GcpSource,
    inner: RwLock<Arc<dyn gcp_auth::TokenProvider>>,
    email: Option<String>,
}

impl GcpAuth {
    async fn new(source: GcpSource, email: Option<String>) -> Result<Self, SheetsError> {
        let inner = source.provider().await?;
        Ok(GcpAuth {
            source,
            inner: RwLock::new(inner),
            email,
        })
    }
}

impl std::fmt::Debug for GcpAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GcpAuth")
            .field("email", &self.email)
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl TokenProvider for GcpAuth {
    async fn token(&self) -> Result<String, SheetsError> {
        let inner = self.inner.read().await.clone();
        let token = inner
            .token(&[SCOPE])
            .await
            .map_err(|e| SheetsError::Auth(e.to_string()))?;
        Ok(token.as_str().to_string())
    }

    /// `gcp_auth` exposes no way to drop its cached token, so rebuild the
    /// provider from the stored source; the next `token()` fetches anew.
    async fn invalidate(&self) {
        if let Ok(fresh) = self.source.provider().await {
            *self.inner.write().await = fresh;
        }
    }

    fn service_account_email(&self) -> Option<String> {
        self.email.clone()
    }
}

/// Minimal shape check of a service-account key, done at construction so
/// misconfiguration fails before the first request.
fn validate_service_account_json(raw: &str) -> Result<String, SheetsError> {
    let v: serde_json::Value = serde_json::from_str(raw).map_err(|e| {
        SheetsError::Credentials(format!("service account JSON is not valid JSON: {e}"))
    })?;
    if v.get("type").and_then(|t| t.as_str()) != Some("service_account") {
        return Err(SheetsError::Credentials(
            "JSON is not a service account key (expected \"type\": \"service_account\")".into(),
        ));
    }
    let email = v
        .get("client_email")
        .and_then(|e| e.as_str())
        .ok_or_else(|| {
            SheetsError::Credentials("service account key is missing client_email".into())
        })?;
    if v.get("private_key").and_then(|k| k.as_str()).is_none() {
        return Err(SheetsError::Credentials(
            "service account key is missing private_key".into(),
        ));
    }
    Ok(email.to_string())
}

/// Build a token provider for the given credentials. Validates key material
/// eagerly; does not exchange a token yet.
pub(crate) async fn provider_for(
    creds: Credentials,
) -> Result<Arc<dyn TokenProvider>, SheetsError> {
    match creds {
        Credentials::AccessToken(t) => {
            if t.trim().is_empty() {
                return Err(SheetsError::Credentials("access token is empty".into()));
            }
            Ok(Arc::new(StaticToken(t)))
        }
        Credentials::ServiceAccountJson(raw) => {
            let email = validate_service_account_json(&raw)?;
            Ok(Arc::new(
                GcpAuth::new(GcpSource::ServiceAccountJson(raw), Some(email)).await?,
            ))
        }
        Credentials::ServiceAccountFile(path) => {
            let raw = std::fs::read_to_string(&path).map_err(|e| {
                SheetsError::Credentials(format!("cannot read {}: {e}", path.display()))
            })?;
            let email = validate_service_account_json(&raw)?;
            Ok(Arc::new(
                GcpAuth::new(GcpSource::ServiceAccountJson(raw), Some(email)).await?,
            ))
        }
        Credentials::ApplicationDefault => Ok(Arc::new(
            GcpAuth::new(GcpSource::ApplicationDefault, None).await?,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A throwaway RSA key generated for tests only (never used with Google).
    const SA_JSON: &str = include_str!("../tests/fixtures/service-account.json");

    #[tokio::test]
    async fn static_token_returns_itself() {
        let p = provider_for(Credentials::AccessToken("tok".into()))
            .await
            .unwrap();
        assert_eq!(p.token().await.unwrap(), "tok");
        assert_eq!(p.service_account_email(), None);
        assert!(!format!("{p:?}").contains("tok"));
        assert_eq!(format!("{p:?}"), "StaticToken(<redacted>)");
    }

    #[tokio::test]
    async fn empty_token_is_rejected() {
        assert!(matches!(
            provider_for(Credentials::AccessToken("  ".into()))
                .await
                .unwrap_err(),
            SheetsError::Credentials(_)
        ));
    }

    #[tokio::test]
    async fn service_account_json_is_validated_and_email_extracted() {
        let p = provider_for(Credentials::ServiceAccountJson(SA_JSON.into()))
            .await
            .unwrap();
        assert_eq!(
            p.service_account_email().as_deref(),
            Some("bot@proj.iam.gserviceaccount.com")
        );
    }

    #[tokio::test]
    async fn invalidate_rebuilds_gcp_provider() {
        let p = provider_for(Credentials::ServiceAccountJson(SA_JSON.into()))
            .await
            .unwrap();
        // Must not panic or deadlock; the provider is swapped for a fresh one.
        p.invalidate().await;
        assert_eq!(
            p.service_account_email().as_deref(),
            Some("bot@proj.iam.gserviceaccount.com")
        );
    }

    #[tokio::test]
    async fn rejects_non_service_account_json() {
        for (raw, expected) in [
            (
                r#"{"type":"authorized_user"}"#,
                "expected \"type\": \"service_account\"",
            ),
            ("not json", "is not valid JSON"),
            (r#"{"type":"service_account"}"#, "missing client_email"),
            (
                r#"{"type":"service_account","client_email":"bot@example.com"}"#,
                "missing private_key",
            ),
        ] {
            let error = provider_for(Credentials::ServiceAccountJson(raw.into()))
                .await
                .unwrap_err();
            assert_eq!(error.code(), "CREDENTIALS");
            assert!(error.to_string().contains(expected), "{error}");
        }
    }

    #[tokio::test]
    async fn service_account_file_is_loaded() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/service-account.json"
        );
        let p = provider_for(Credentials::ServiceAccountFile(path.into()))
            .await
            .unwrap();
        assert_eq!(
            p.service_account_email().as_deref(),
            Some("bot@proj.iam.gserviceaccount.com")
        );
    }

    #[tokio::test]
    async fn missing_file_is_a_credentials_error() {
        let err = provider_for(Credentials::ServiceAccountFile(
            "/nonexistent/sa.json".into(),
        ))
        .await
        .unwrap_err();
        assert!(matches!(err, SheetsError::Credentials(m) if m.contains("/nonexistent/sa.json")));
    }
}
