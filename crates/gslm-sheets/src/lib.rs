//! Google Sheets client for gslm (ADR-0004): hand-rolled REST over `reqwest`
//! with `gcp_auth` for service-account / ADC tokens.

mod auth;
mod client;
mod error;
mod range;

pub use auth::{Credentials, TokenProvider};
pub use client::{DEFAULT_CONNECT_TIMEOUT, DEFAULT_TIMEOUT, SheetsClient, SheetsClientBuilder};
pub use error::SheetsError;
pub use gslm_core::Table;

/// Only scope this client ever requests.
pub const SCOPE: &str = "https://www.googleapis.com/auth/spreadsheets";
/// Production Sheets API origin; overridable for tests.
pub const DEFAULT_BASE_URL: &str = "https://sheets.googleapis.com";
