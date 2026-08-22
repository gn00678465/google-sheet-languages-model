//! napi adapters for the Rust CLI orchestration crate.

use crate::config::{ConfigTarget, resolved_target};
use gslm_cli::{ColorChoice, PullOptions, PushOptions, RunOptions, SheetsOverride};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::path::PathBuf;

fn to_js(error: gslm_cli::CliError) -> Error {
    Error::new(
        Status::GenericFailure,
        format!("[{}] {error}", error.code()),
    )
}

/// Options for an embedded CLI invocation. Network overrides are intentionally
/// not public JS API; fixture tests use `GSLM_CLI_BASE_URL` and
/// `GSLM_CLI_ACCESS_TOKEN` as an environment bridge.
#[napi(object)]
#[derive(Default)]
pub struct RunCliOptions {
    pub cwd: Option<String>,
    pub is_tty: Option<bool>,
    pub color: Option<String>,
}

#[napi(object)]
#[derive(Default)]
/// Controls a high-level pull call from JavaScript.
pub struct HighLevelPullOptions {
    pub dry_run: Option<bool>,
    pub force: Option<bool>,
    /// Test/embedding-only Sheets origin; production callers omit it.
    pub base_url: Option<String>,
    /// Test/embedding-only static token; production callers omit it.
    pub access_token: Option<String>,
}

#[napi(object)]
#[derive(Default)]
/// Controls a high-level push call from JavaScript.
pub struct HighLevelPushOptions {
    pub dry_run: Option<bool>,
    pub force: Option<bool>,
    pub strict: Option<bool>,
    /// Test/embedding-only Sheets origin; production callers omit it.
    pub base_url: Option<String>,
    /// Test/embedding-only static token; production callers omit it.
    pub access_token: Option<String>,
}

#[napi(object)]
/// A Catalog file affected by pull.
pub struct JsFileSummary {
    pub locale: String,
    pub path: String,
    pub keys: u32,
    pub outcome: Option<String>,
}

#[napi(object)]
/// Data-only summary returned by a high-level pull.
pub struct JsPullSummary {
    pub target: String,
    pub files: Vec<JsFileSummary>,
    pub created: u32,
    pub updated: u32,
    pub unchanged: u32,
}

#[napi(object)]
/// One Locale's key count in a push result.
pub struct JsLocaleKeyCount {
    pub locale: String,
    pub keys: u32,
}

#[napi(object)]
/// Data-only summary returned by a high-level push.
pub struct JsPushSummary {
    pub target: String,
    pub rows: u32,
    pub columns: u32,
    pub locale_keys: Vec<JsLocaleKeyCount>,
    pub orphan_keys: Vec<String>,
    pub warnings: Vec<String>,
}

impl From<gslm_cli::PullSummary> for JsPullSummary {
    fn from(value: gslm_cli::PullSummary) -> Self {
        Self {
            target: value.target,
            files: value
                .files
                .into_iter()
                .map(|file| JsFileSummary {
                    locale: file.locale,
                    path: file.path.to_string_lossy().into_owned(),
                    keys: file.keys as u32,
                    outcome: file
                        .outcome
                        .map(|outcome| format!("{outcome:?}").to_lowercase()),
                })
                .collect(),
            created: value.created as u32,
            updated: value.updated as u32,
            unchanged: value.unchanged as u32,
        }
    }
}

impl From<gslm_cli::PushSummary> for JsPushSummary {
    fn from(value: gslm_cli::PushSummary) -> Self {
        Self {
            target: value.target,
            rows: value.rows as u32,
            columns: value.columns as u32,
            locale_keys: value
                .locale_keys
                .into_iter()
                .map(|(locale, keys)| JsLocaleKeyCount {
                    locale,
                    keys: keys as u32,
                })
                .collect(),
            orphan_keys: value.orphan_keys,
            warnings: value.warnings,
        }
    }
}

fn color(value: Option<String>) -> Option<ColorChoice> {
    match value.as_deref() {
        Some("always") => Some(ColorChoice::Always),
        Some("never") => Some(ColorChoice::Never),
        _ => Some(ColorChoice::Auto),
    }
}

fn test_sheets_override() -> SheetsOverride {
    SheetsOverride {
        base_url: std::env::var("GSLM_CLI_BASE_URL")
            .ok()
            .filter(|value| !value.is_empty()),
        access_token: std::env::var("GSLM_CLI_ACCESS_TOKEN")
            .ok()
            .filter(|value| !value.is_empty()),
    }
}

/// Run the same clap-driven CLI as `bin/gslm.js`, returning its shell exit
/// code instead of throwing command errors.
#[napi]
pub async fn run_cli(argv: Vec<String>, options: Option<RunCliOptions>) -> Result<i32> {
    let options = options.unwrap_or_default();
    let options = RunOptions {
        cwd: options
            .cwd
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))),
        stdout: Box::new(std::io::stdout()),
        stderr: Box::new(std::io::stderr()),
        sheets: test_sheets_override(),
        color: color(options.color),
        is_tty: options.is_tty,
        version: Some(env!("GSLM_PACKAGE_VERSION")),
        ..RunOptions::default()
    };
    Ok(gslm_cli::run_async(argv, options).await)
}

/// Pull one Target returned by `loadConfig` without reparsing environment
/// credentials. The hidden credential handle remains entirely in Rust.
#[napi]
pub async fn pull(
    target: ConfigTarget,
    options: Option<HighLevelPullOptions>,
) -> Result<JsPullSummary> {
    let target = resolved_target(target)?;
    let options = options.unwrap_or_default();
    gslm_cli::pull(
        &target,
        PullOptions {
            dry_run: options.dry_run.unwrap_or(false),
            force: options.force.unwrap_or(false),
            sheets: SheetsOverride {
                base_url: options.base_url,
                access_token: options.access_token,
            },
        },
    )
    .await
    .map(Into::into)
    .map_err(to_js)
}

/// Push one Target returned by `loadConfig` without reparsing environment
/// credentials. The hidden credential handle remains entirely in Rust.
#[napi]
pub async fn push(
    target: ConfigTarget,
    options: Option<HighLevelPushOptions>,
) -> Result<JsPushSummary> {
    let target = resolved_target(target)?;
    let options = options.unwrap_or_default();
    gslm_cli::push(
        &target,
        PushOptions {
            dry_run: options.dry_run.unwrap_or(false),
            force: options.force.unwrap_or(false),
            strict: options.strict.unwrap_or(false),
            sheets: SheetsOverride {
                base_url: options.base_url,
                access_token: options.access_token,
            },
        },
    )
    .await
    .map(Into::into)
    .map_err(to_js)
}
