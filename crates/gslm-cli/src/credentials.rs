use gslm_config::CredentialsSource;
use gslm_sheets::Credentials;
use std::path::PathBuf;

/// Safe, human-facing facts about a credential source.
///
/// It intentionally excludes service-account JSON so it can be used in
/// diagnostics and across the N-API boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialDetails {
    pub kind: &'static str,
    pub label: &'static str,
    pub cache_key: String,
    pub path: Option<PathBuf>,
    pub env_name: Option<String>,
}

/// Describe a credential source without exposing its secret value.
pub fn credential_details(source: &CredentialsSource) -> CredentialDetails {
    match source {
        CredentialsSource::File(path) => CredentialDetails {
            kind: "file",
            label: "檔案",
            cache_key: format!("file:{}", path.display()),
            path: Some(path.clone()),
            env_name: None,
        },
        CredentialsSource::Json { env_name, .. } => CredentialDetails {
            kind: "json",
            label: "環境變數 JSON",
            cache_key: format!("env:{env_name}"),
            path: None,
            env_name: Some(env_name.clone()),
        },
        CredentialsSource::ApplicationDefault => CredentialDetails {
            kind: "adc",
            label: "Application Default Credentials",
            cache_key: "adc".into(),
            path: None,
            env_name: None,
        },
    }
}

/// Build the Sheets authentication input from a resolved credential source.
pub fn sheets_credentials(source: &CredentialsSource) -> Credentials {
    match source {
        CredentialsSource::File(path) => Credentials::ServiceAccountFile(path.clone()),
        CredentialsSource::Json { value, .. } => Credentials::ServiceAccountJson(value.clone()),
        CredentialsSource::ApplicationDefault => Credentials::ApplicationDefault,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn details_exclude_json_credentials_but_keep_the_source_name() {
        let details = credential_details(&CredentialsSource::Json {
            env_name: "GSLM_SERVICE_ACCOUNT".into(),
            value: "never-expose-this-secret".into(),
        });

        assert_eq!(details.kind, "json");
        assert_eq!(details.label, "環境變數 JSON");
        assert_eq!(details.env_name.as_deref(), Some("GSLM_SERVICE_ACCOUNT"));
        assert!(!format!("{details:?}").contains("never-expose-this-secret"));
    }
}
