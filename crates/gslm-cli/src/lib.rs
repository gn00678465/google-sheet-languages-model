//! Command-line orchestration for gslm.
//!
//! The crate deliberately exposes one command boundary, [`run`], so the Node
//! binding and a future standalone executable exercise exactly the same flow.

mod args;
mod catalog_fs;
mod credentials;
mod init;
mod pull;
mod push;
mod report;

pub use catalog_fs::{Shape, WriteOutcome, detect_shape, render_path};
pub use credentials::{CredentialDetails, credential_details, sheets_credentials};
pub use pull::{FileSummary, PullOptions, PullSummary, pull};
pub use push::{PushOptions, PushSummary, push};

use args::{Cli, Command, FieldOverrides};
use clap::error::ErrorKind;
use clap::{Arg, ArgAction, CommandFactory, FromArgMatches};
use gslm_config::{CredentialsSource, LoadOptions, Overrides, ResolvedTarget};
use gslm_core::Format;
use gslm_sheets::{Credentials, SheetsClient, SheetsError};
use report::Reporter;
use std::collections::{BTreeMap, HashMap};
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use thiserror::Error;

/// Explicit choices for diagnostic colour output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

/// Overrides for Sheets connections, primarily for embedders and tests.
#[derive(Debug, Clone, Default)]
pub struct SheetsOverride {
    pub base_url: Option<String>,
    pub access_token: Option<String>,
}

/// Dependencies supplied by an embedding host instead of read globally.
pub struct RunOptions {
    pub cwd: PathBuf,
    pub env: Option<BTreeMap<String, String>>,
    pub stdout: Box<dyn Write + Send>,
    pub stderr: Box<dyn Write + Send>,
    pub sheets: SheetsOverride,
    pub color: Option<ColorChoice>,
    pub is_tty: Option<bool>,
    /// Version shown by clap. Embedders should provide their package version.
    pub version: Option<&'static str>,
}

impl Default for RunOptions {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            env: None,
            stdout: Box::new(std::io::stdout()),
            stderr: Box::new(std::io::stderr()),
            sheets: SheetsOverride::default(),
            color: None,
            is_tty: None,
            version: None,
        }
    }
}

/// Errors which the CLI presents as a single coded Traditional-Chinese line.
#[derive(Debug, Error)]
pub enum CliError {
    #[error("{0}")]
    Config(#[from] gslm_config::ConfigError),
    #[error("無法讀取 Catalog {path}：{reason}")]
    Catalog { path: PathBuf, reason: String },
    #[error("轉換資料失敗：{}", core_message(.0))]
    Core(#[from] gslm_core::ConversionError),
    #[error("{}", sheets_message(.0))]
    Sheets(#[from] SheetsError),
    #[error("Sheet 為空，本地有 {local_keys} 個 key；若確定要清空請加 --force")]
    PullEmptySheet { local_keys: usize },
    #[error("所有本地 Catalog 都是空的；若確定要清空 Sheet 請加 --force")]
    PushEmptyLocal,
    #[error("嚴格模式拒絕 push：{}", reasons.join("；"))]
    PushStrict { reasons: Vec<String> },
    #[error("無法存取 {path}：{source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("請經由 gslm 的 JS 入口執行 migrate")]
    MigrateViaJs,
}

fn sheets_message(error: &SheetsError) -> String {
    match error {
        SheetsError::WriteAfterClearFailed { .. } => "Tab 已被清空但寫入失敗，請重試 push".into(),
        SheetsError::Credentials(message) => format!("Google Sheets 憑證無效：{message}"),
        SheetsError::Auth(_) => "無法取得 Google Sheets 存取權杖".into(),
        SheetsError::PermissionDenied { sheet_id, .. } => {
            format!("沒有 Sheet {sheet_id} 的存取權限；請將服務帳號設為編輯者")
        }
        SheetsError::SheetNotFound { sheet_id } => format!("找不到 Sheet {sheet_id}"),
        SheetsError::TabNotFound { sheet_id, tab } => {
            format!("找不到 Sheet {sheet_id} 中的 Tab {tab}")
        }
        SheetsError::RateLimited => "Google Sheets API 已達速率限制，請稍後重試".into(),
        SheetsError::ServerError { status } => {
            format!("Google Sheets API 伺服器錯誤（HTTP {status}），請稍後重試")
        }
        SheetsError::Http { status, .. } => format!("Google Sheets API 回應 HTTP {status}"),
        SheetsError::Network(_) => "無法連線至 Google Sheets API".into(),
        SheetsError::InvalidResponse(_) => "Google Sheets API 回應格式無效".into(),
    }
}

pub(crate) fn core_message(error: &gslm_core::ConversionError) -> String {
    use gslm_core::ConversionError;

    match error {
        ConversionError::NotAnObject(kind) => format!("Catalog 最外層必須是物件，目前為 {kind}"),
        ConversionError::NumericKeySegment(key) => format!("key 片段不可為數字：{key}"),
        ConversionError::EmptySeparator => "key 分隔符不可為空字串".into(),
        ConversionError::ArrayNotSupported(key) => format!("Catalog 不支援陣列（key：{key}）"),
        ConversionError::NonStringTranslation { key, kind } => {
            format!("翻譯值必須是字串，目前為 {kind}（key：{key}）")
        }
        ConversionError::KeyConflict { key } => format!("key {key} 與巢狀 key 衝突"),
        ConversionError::DuplicateFlatKey { key } => {
            format!("巢狀與扁平寫法產生重複 key：{key}")
        }
        ConversionError::EmptySheet => "Sheet 為空，缺少標題列".into(),
        ConversionError::LocaleNotInHeader { locale, available } => {
            format!(
                "Sheet 標題列沒有 Locale {locale}（可用：{}）",
                available.join(", ")
            )
        }
        ConversionError::DuplicateKey { key, row } => format!("key {key} 在第 {row} 列重複"),
        ConversionError::UnknownLocale(locale) => format!("Locale {locale} 不屬於此 Model"),
    }
}

impl CliError {
    /// Stable error code consumed by scripts and displayed to people.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Config(error) => error.code(),
            Self::Catalog { .. } => "CATALOG",
            Self::Core(_) => "CONVERSION",
            Self::Sheets(error) => error.code(),
            Self::PullEmptySheet { .. } => "PULL_EMPTY_SHEET",
            Self::PushEmptyLocal => "PUSH_EMPTY_LOCAL",
            Self::PushStrict { .. } => "PUSH_STRICT",
            Self::Io { .. } => "IO",
            Self::MigrateViaJs => "MIGRATE_JS_ONLY",
        }
    }
}

/// Execute one gslm command. `argv[0]` is the program name.
pub fn run(argv: Vec<String>, options: RunOptions) -> i32 {
    match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime.block_on(run_async(argv, options)),
        Err(error) => {
            let mut stderr = options.stderr;
            let _ = writeln!(stderr, "error: [RUNTIME] 無法建立執行環境：{error}");
            1
        }
    }
}

/// Async counterpart to [`run`], for hosts that already own a Tokio runtime.
pub async fn run_async(argv: Vec<String>, mut options: RunOptions) -> i32 {
    let parser = cli_command(&options);
    let cli = match parser
        .try_get_matches_from(argv)
        .and_then(|matches| Cli::from_arg_matches(&matches))
    {
        Ok(cli) => cli,
        Err(error) => {
            let display_only = matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            );
            let writer: &mut (dyn Write + Send) = if display_only {
                &mut *options.stdout
            } else {
                &mut *options.stderr
            };
            let _ = write!(writer, "{error}");
            return if display_only { 0 } else { 2 };
        }
    };
    if cli.command.is_none() {
        let mut command = cli_command(&options);
        let _ = command.write_long_help(&mut options.stderr);
        let _ = writeln!(options.stderr);
        return 2;
    }

    let result = match cli.command.as_ref().expect("checked above") {
        Command::Schema => {
            let rendered =
                serde_json::to_string_pretty(&gslm_config::schema()).expect("schema serializes");
            let _ = writeln!(options.stdout, "{rendered}");
            Ok(())
        }
        Command::Init(init_args) => init::run(&options.cwd, init_args).map(|path| {
            if !cli.common.quiet {
                let _ = writeln!(options.stderr, "已建立設定檔：{}", path.display());
            }
        }),
        Command::Migrate => Err(CliError::MigrateViaJs),
        Command::Pull(command) => {
            let command = cli.sync_command(command.clone());
            run_sync_command(&command, true, &mut options).await
        }
        Command::Push(command) => {
            let command = cli.push_command(command.clone());
            run_sync_command(&command, false, &mut options).await
        }
    };
    match result {
        Ok(()) => 0,
        Err(error) => {
            let _ = writeln!(options.stderr, "error: [{}] {error}", error.code());
            1
        }
    }
}

async fn run_sync_command(
    command: &args::SyncCommand,
    is_pull: bool,
    options: &mut RunOptions,
) -> Result<(), CliError> {
    let loaded = load_config(command, options)?;
    let mut reporter = Reporter::new(
        &mut options.stdout,
        &mut options.stderr,
        command.quiet,
        command.verbose,
        command.color.or(options.color),
        options.is_tty,
        options
            .env
            .as_ref()
            .map(|env| env.contains_key("NO_COLOR"))
            .unwrap_or_else(|| std::env::var_os("NO_COLOR").is_some()),
    );
    for warning in &loaded.warnings {
        reporter.warning(warning);
    }
    let mut clients = HashMap::<String, SheetsClient>::new();
    for target in loaded.targets {
        reporter.target(&target);
        reporter.request_details(&target, is_pull);
        let key = client_key(&target, &options.sheets);
        let client = if let Some(client) = clients.get(&key) {
            client.clone()
        } else {
            let client = build_client(&target.credentials, &options.sheets).await?;
            clients.insert(key, client.clone());
            client
        };
        if is_pull {
            let summary = pull::pull_with_client(
                &target,
                pull::PullOptions {
                    dry_run: command.dry_run,
                    force: command.force,
                    sheets: options.sheets.clone(),
                },
                &client,
            )
            .await?;
            reporter.pull_summary(&summary, command.dry_run);
        } else {
            let summary = push::push_with_client(
                &target,
                push::PushOptions {
                    dry_run: command.dry_run,
                    force: command.force,
                    strict: command.strict,
                    sheets: options.sheets.clone(),
                },
                &client,
            )
            .await?;
            for warning in &summary.warnings {
                reporter.warning(warning);
            }
            reporter.push_summary(&summary, command.dry_run);
        }
    }
    Ok(())
}

fn load_config(
    command: &args::SyncCommand,
    options: &RunOptions,
) -> Result<gslm_config::ResolvedConfig, CliError> {
    let targets = command.targets.clone();
    gslm_config::load(LoadOptions {
        cwd: options.cwd.clone(),
        config_path: command.config.clone(),
        env: options.env.clone().unwrap_or_else(process_environment),
        overrides: command.overrides.to_config(),
        targets: (!targets.is_empty()).then_some(targets),
        load_dotenv: !command.no_dotenv,
    })
    .map_err(Into::into)
}

fn cli_command(options: &RunOptions) -> clap::Command {
    Cli::command()
        .name("gslm")
        .bin_name("gslm")
        .version(options.version.unwrap_or(env!("CARGO_PKG_VERSION")))
        .arg(
            Arg::new("help")
                .short('h')
                .long("help")
                .global(true)
                .action(ArgAction::Help)
                .help("顯示此說明")
                .help_heading("選項"),
        )
        .arg(
            Arg::new("version")
                .short('V')
                .long("version")
                .action(ArgAction::Version)
                .help("顯示版本")
                .help_heading("選項"),
        )
}

fn process_environment() -> BTreeMap<String, String> {
    environment_from_os(std::env::vars_os())
}

fn environment_from_os<I>(environment: I) -> BTreeMap<String, String>
where
    I: IntoIterator<Item = (OsString, OsString)>,
{
    environment
        .into_iter()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .collect()
}

fn client_key(target: &ResolvedTarget, override_options: &SheetsOverride) -> String {
    let credentials = credential_details(&target.credentials).cache_key;
    format!(
        "{credentials}:{}:{:?}",
        target.sheet, override_options.base_url
    )
}

pub(crate) async fn build_client(
    credentials: &CredentialsSource,
    override_options: &SheetsOverride,
) -> Result<SheetsClient, CliError> {
    let credentials = match &override_options.access_token {
        Some(token) => Credentials::AccessToken(token.clone()),
        None => sheets_credentials(credentials),
    };
    let mut builder = SheetsClient::builder(credentials);
    if let Some(base_url) = &override_options.base_url {
        builder = builder.base_url(base_url);
    }
    builder.build().await.map_err(Into::into)
}

impl FieldOverrides {
    fn to_config(&self) -> Overrides {
        Overrides {
            sheet: self.sheet.clone(),
            tab: self.tab.clone(),
            locales: self.locales.clone(),
            path: self.path.clone(),
            format: self.format.map(|format| match format {
                args::FormatArg::Nest => Format::Nest,
                args::FormatArg::Flat => Format::Flat,
            }),
            key_separator: self.key_separator.clone(),
            credentials: self.credentials.clone(),
            credentials_json: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn non_unicode_process_environment_entries_are_ignored() {
        use std::os::unix::ffi::OsStringExt;

        let environment = environment_from_os([
            (OsString::from("VALID"), OsString::from("value")),
            (OsString::from_vec(vec![0xFF]), OsString::from("value")),
            (
                OsString::from("INVALID_VALUE"),
                OsString::from_vec(vec![0xFF]),
            ),
        ]);

        assert_eq!(
            environment,
            BTreeMap::from([("VALID".into(), "value".into())])
        );
    }

    #[test]
    fn sheets_failures_have_actionable_cli_messages_and_stable_codes() {
        let cases = vec![
            (
                SheetsError::Credentials("bad key".into()),
                "CREDENTIALS",
                "Google Sheets 憑證無效：bad key",
            ),
            (
                SheetsError::Auth("expired".into()),
                "AUTH",
                "無法取得 Google Sheets 存取權杖",
            ),
            (
                SheetsError::PermissionDenied {
                    sheet_id: "sheet".into(),
                    service_account_email: None,
                },
                "PERMISSION_DENIED",
                "沒有 Sheet sheet 的存取權限；請將服務帳號設為編輯者",
            ),
            (
                SheetsError::SheetNotFound {
                    sheet_id: "sheet".into(),
                },
                "SHEET_NOT_FOUND",
                "找不到 Sheet sheet",
            ),
            (
                SheetsError::TabNotFound {
                    sheet_id: "sheet".into(),
                    tab: "Tab".into(),
                },
                "TAB_NOT_FOUND",
                "找不到 Sheet sheet 中的 Tab Tab",
            ),
            (
                SheetsError::RateLimited,
                "RATE_LIMITED",
                "Google Sheets API 已達速率限制，請稍後重試",
            ),
            (
                SheetsError::ServerError { status: 503 },
                "SERVER_ERROR",
                "Google Sheets API 伺服器錯誤（HTTP 503），請稍後重試",
            ),
            (
                SheetsError::Http {
                    status: 418,
                    message: "teapot".into(),
                },
                "HTTP",
                "Google Sheets API 回應 HTTP 418",
            ),
            (
                SheetsError::Network("offline".into()),
                "NETWORK",
                "無法連線至 Google Sheets API",
            ),
            (
                SheetsError::InvalidResponse("not JSON".into()),
                "INVALID_RESPONSE",
                "Google Sheets API 回應格式無效",
            ),
            (
                SheetsError::WriteAfterClearFailed {
                    source: Box::new(SheetsError::Network("offline".into())),
                },
                "WRITE_AFTER_CLEAR_FAILED",
                "Tab 已被清空但寫入失敗，請重試 push",
            ),
        ];

        for (source, code, message) in cases {
            let error = CliError::from(source);
            assert_eq!(error.code(), code);
            assert_eq!(error.to_string(), message);
        }
    }

    #[test]
    fn conversion_failures_have_actionable_cli_messages_and_a_shared_code() {
        use gslm_core::ConversionError;

        let cases = vec![
            (
                ConversionError::NotAnObject("array"),
                "Catalog 最外層必須是物件，目前為 array",
            ),
            (
                ConversionError::NumericKeySegment("0".into()),
                "key 片段不可為數字：0",
            ),
            (ConversionError::EmptySeparator, "key 分隔符不可為空字串"),
            (
                ConversionError::ArrayNotSupported("items".into()),
                "Catalog 不支援陣列（key：items）",
            ),
            (
                ConversionError::NonStringTranslation {
                    key: "count".into(),
                    kind: "number",
                },
                "翻譯值必須是字串，目前為 number（key：count）",
            ),
            (
                ConversionError::KeyConflict { key: "a".into() },
                "key a 與巢狀 key 衝突",
            ),
            (
                ConversionError::DuplicateFlatKey { key: "a.b".into() },
                "巢狀與扁平寫法產生重複 key：a.b",
            ),
            (ConversionError::EmptySheet, "Sheet 為空，缺少標題列"),
            (
                ConversionError::LocaleNotInHeader {
                    locale: "fr".into(),
                    available: vec!["en".into(), "zh-TW".into()],
                },
                "Sheet 標題列沒有 Locale fr（可用：en, zh-TW）",
            ),
            (
                ConversionError::DuplicateKey {
                    key: "title".into(),
                    row: 3,
                },
                "key title 在第 3 列重複",
            ),
            (
                ConversionError::UnknownLocale("fr".into()),
                "Locale fr 不屬於此 Model",
            ),
        ];

        for (source, message) in cases {
            let error = CliError::from(source);
            assert_eq!(error.code(), "CONVERSION");
            assert_eq!(error.to_string(), format!("轉換資料失敗：{message}"));
        }
    }
}
