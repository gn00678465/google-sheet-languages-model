use thiserror::Error;

/// Everything that can go wrong talking to Google Sheets, classified so that
/// callers (CLI, SDK) can act on the kind rather than parse messages.
#[derive(Debug, Error)]
pub enum SheetsError {
    #[error("invalid credentials: {0}")]
    Credentials(String),
    #[error("could not obtain an access token: {0}")]
    Auth(String),
    #[error(
        "permission denied for sheet {sheet_id}: share the sheet with {} (Editor) and retry",
        service_account_email.as_deref().unwrap_or("the service account email")
    )]
    PermissionDenied {
        sheet_id: String,
        service_account_email: Option<String>,
    },
    #[error("sheet {sheet_id} not found: check the spreadsheet ID in the URL")]
    SheetNotFound { sheet_id: String },
    #[error("tab {tab:?} not found in sheet {sheet_id}")]
    TabNotFound { sheet_id: String, tab: String },
    #[error("rate limited by Google Sheets API (HTTP 429); retry later")]
    RateLimited,
    #[error("Google Sheets API server error (HTTP {status}); retry later")]
    ServerError { status: u16 },
    #[error("Google Sheets API error (HTTP {status}): {message}")]
    Http { status: u16, message: String },
    #[error("network error: {0}")]
    Network(String),
    #[error("unexpected response from Google Sheets API: {0}")]
    InvalidResponse(String),
    #[error("the tab was cleared but writing the new values failed; push again: {source}")]
    WriteAfterClearFailed { source: Box<SheetsError> },
}

impl SheetsError {
    /// Stable, language-neutral code (also surfaced as `error.code` in JS).
    pub fn code(&self) -> &'static str {
        match self {
            SheetsError::Credentials(_) => "CREDENTIALS",
            SheetsError::Auth(_) => "AUTH",
            SheetsError::PermissionDenied { .. } => "PERMISSION_DENIED",
            SheetsError::SheetNotFound { .. } => "SHEET_NOT_FOUND",
            SheetsError::TabNotFound { .. } => "TAB_NOT_FOUND",
            SheetsError::RateLimited => "RATE_LIMITED",
            SheetsError::ServerError { .. } => "SERVER_ERROR",
            SheetsError::Http { .. } => "HTTP",
            SheetsError::Network(_) => "NETWORK",
            SheetsError::InvalidResponse(_) => "INVALID_RESPONSE",
            SheetsError::WriteAfterClearFailed { .. } => "WRITE_AFTER_CLEAR_FAILED",
        }
    }

    /// True for 429 / 5xx / network: a retry may succeed.
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            SheetsError::RateLimited | SheetsError::ServerError { .. } | SheetsError::Network(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::SheetsError;

    #[test]
    fn stable_codes_and_retry_classification_cover_every_error_kind() {
        let errors = vec![
            (
                SheetsError::Credentials("bad key".into()),
                "CREDENTIALS",
                false,
            ),
            (SheetsError::Auth("expired".into()), "AUTH", false),
            (
                SheetsError::PermissionDenied {
                    sheet_id: "sheet".into(),
                    service_account_email: None,
                },
                "PERMISSION_DENIED",
                false,
            ),
            (
                SheetsError::SheetNotFound {
                    sheet_id: "sheet".into(),
                },
                "SHEET_NOT_FOUND",
                false,
            ),
            (
                SheetsError::TabNotFound {
                    sheet_id: "sheet".into(),
                    tab: "Tab".into(),
                },
                "TAB_NOT_FOUND",
                false,
            ),
            (SheetsError::RateLimited, "RATE_LIMITED", true),
            (
                SheetsError::ServerError { status: 503 },
                "SERVER_ERROR",
                true,
            ),
            (
                SheetsError::Http {
                    status: 418,
                    message: "teapot".into(),
                },
                "HTTP",
                false,
            ),
            (SheetsError::Network("offline".into()), "NETWORK", true),
            (
                SheetsError::InvalidResponse("not JSON".into()),
                "INVALID_RESPONSE",
                false,
            ),
            (
                SheetsError::WriteAfterClearFailed {
                    source: Box::new(SheetsError::Network("offline".into())),
                },
                "WRITE_AFTER_CLEAR_FAILED",
                false,
            ),
        ];

        for (error, code, transient) in errors {
            assert_eq!(error.code(), code);
            assert_eq!(error.is_transient(), transient);
            assert!(!error.to_string().is_empty());
        }
    }
}
