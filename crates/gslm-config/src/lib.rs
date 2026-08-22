//! Discover, parse, validate, and resolve gslm configuration files.
//!
//! The public seam is [`load`]. It deliberately returns fully expanded Targets
//! so callers never need to know whether a value came from a file, the
//! environment, or a command-line override.

use gslm_core::Format;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};

const CONFIG_NAMES: [&str; 3] = ["gslm.toml", "gslm.jsonc", "gslm.json"];
const LEGACY_NAMES: [&str; 4] = [
    "gslm.config.js",
    "gslm.config.ts",
    "gslm.config.mjs",
    "gslm.config.cjs",
];
const SCHEMA_ID: &str = "https://gn00678465.github.io/google-sheet-languages-model/schema/v1.json";

/// A source for the service-account credentials of a Target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialsSource {
    File(PathBuf),
    /// `value` is retained in Rust only. Bindings must serialize just
    /// `env_name`, never the credential JSON.
    Json {
        env_name: String,
        value: String,
    },
    ApplicationDefault,
}

/// A fully expanded Target ready for pull or push.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedTarget {
    pub name: String,
    pub sheet: String,
    pub tab: String,
    /// The first Locale is the Source locale.
    pub locales: Vec<String>,
    /// An absolute path template which still contains `{locale}`.
    pub path: PathBuf,
    pub format: Format,
    pub key_separator: String,
    pub credentials: CredentialsSource,
}

/// Output from [`load`]. Warnings are informational, such as multiple config
/// formats in one directory; callers may decide how to present them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedConfig {
    pub config_path: Option<PathBuf>,
    pub targets: Vec<ResolvedTarget>,
    pub warnings: Vec<String>,
}

/// Values supplied by a CLI instead of a config file.
#[derive(Debug, Clone, Default)]
pub struct Overrides {
    pub sheet: Option<String>,
    pub tab: Option<String>,
    pub locales: Option<Vec<String>>,
    pub path: Option<String>,
    pub format: Option<Format>,
    pub key_separator: Option<String>,
    pub credentials: Option<String>,
    pub credentials_json: Option<String>,
}

impl Overrides {
    fn has_target_values(&self) -> bool {
        self.sheet.is_some()
            || self.tab.is_some()
            || self.locales.is_some()
            || self.path.is_some()
            || self.format.is_some()
            || self.key_separator.is_some()
    }

    fn validate_credentials(&self) -> Result<(), ConfigError> {
        if self.credentials.is_some() && self.credentials_json.is_some() {
            return Err(ConfigError::Invalid {
                path: None,
                field: "credentials".into(),
                message: "只能提供 credentials 或 credentials_json 其中之一".into(),
            });
        }
        Ok(())
    }
}

/// Input to [`load`]. `env` defaults to the process environment and is a map
/// so tests and embedders can supply an isolated environment.
#[derive(Debug, Clone)]
pub struct LoadOptions {
    pub cwd: PathBuf,
    pub config_path: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
    pub overrides: Overrides,
    pub targets: Option<Vec<String>>,
    pub load_dotenv: bool,
}

impl Default for LoadOptions {
    fn default() -> Self {
        Self {
            cwd: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            config_path: None,
            env: env::vars().collect(),
            overrides: Overrides::default(),
            targets: None,
            load_dotenv: true,
        }
    }
}

/// Stable errors returned by [`load`].
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("找不到設定檔；從 {start} 開始搜尋：{searched:?}")]
    NotFound {
        start: PathBuf,
        searched: Vec<PathBuf>,
    },
    #[error("找到舊版可執行設定檔 {path}；請執行 gslm migrate")]
    Legacy { path: PathBuf },
    #[error("不支援的設定檔格式：{path}（僅支援 .toml、.jsonc、.json）")]
    Unsupported { path: PathBuf },
    #[error(
        "無法解析設定檔 {path}{location}: {message}",
        location = self.location()
    )]
    Parse {
        path: PathBuf,
        line: Option<usize>,
        column: Option<usize>,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
    #[error("不支援設定檔 version={found}；請升級 gslm")]
    UnsupportedVersion { found: String },
    #[error("設定檔缺少必填的 version = 1")]
    MissingVersion,
    #[error(
        "無效設定{path_display}的 {field}：{message}",
        path_display = self.path_display()
    )]
    Invalid {
        path: Option<PathBuf>,
        field: String,
        message: String,
    },
    #[error(
        "未知欄位 {field}{suggestion_display}",
        suggestion_display = self.suggestion_display()
    )]
    UnknownField {
        field: String,
        suggestion: Option<String>,
    },
    #[error("多個 Target 時，欄位覆寫必須搭配恰好一個 --target（可用：{available:?}）")]
    AmbiguousOverride { available: Vec<String> },
    #[error("找不到 Target {name}（可用：{available:?}）")]
    UnknownTarget {
        name: String,
        available: Vec<String>,
    },
    #[error("缺少或為空的環境變數 {name}")]
    MissingEnv { name: String },
}

impl ConfigError {
    /// A stable, machine-readable error code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "CONFIG_NOT_FOUND",
            Self::Legacy { .. } => "CONFIG_LEGACY",
            Self::Unsupported { .. } => "CONFIG_UNSUPPORTED",
            Self::Parse { .. } => "CONFIG_PARSE",
            Self::UnsupportedVersion { .. } => "CONFIG_UNSUPPORTED_VERSION",
            Self::MissingVersion
            | Self::Invalid { .. }
            | Self::UnknownField { .. }
            | Self::AmbiguousOverride { .. }
            | Self::UnknownTarget { .. }
            | Self::MissingEnv { .. } => "CONFIG_INVALID",
        }
    }
}

impl ConfigError {
    fn location(&self) -> String {
        match self {
            Self::Parse {
                line: Some(line),
                column: Some(column),
                ..
            } => format!("（第 {line} 行，第 {column} 欄）"),
            _ => String::new(),
        }
    }

    fn path_display(&self) -> String {
        match self {
            Self::Invalid {
                path: Some(path), ..
            } => format!(" {}", path.display()),
            _ => String::new(),
        }
    }

    fn suggestion_display(&self) -> String {
        match self {
            Self::UnknownField {
                suggestion: Some(suggestion),
                ..
            } => format!("；{suggestion}"),
            _ => String::new(),
        }
    }
}

/// Load a config file, apply environment and CLI overrides, and expand it into
/// one or more Targets. The precedence is CLI overrides > `GSLM_*` > file >
/// built-in defaults.
pub fn load(mut options: LoadOptions) -> Result<ResolvedConfig, ConfigError> {
    options.cwd = absolute_path(
        &options.cwd,
        &env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
    );
    if options.load_dotenv {
        load_dotenv(&options.cwd, &mut options.env)?;
    }

    let env_overrides = environment_overrides(&options.env, &options.cwd)?;
    options.overrides.validate_credentials()?;

    let discovery = discover(&options.cwd, options.config_path.as_deref())?;
    let (config_path, mut warnings, mut drafts, searched) = match discovery {
        Discovery::Found { path, warnings } => {
            let raw = read_config(&path)?;
            let config_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
            let drafts = drafts_from_raw(raw, &config_dir)?;
            (Some(path), warnings, drafts, Vec::new())
        }
        Discovery::Absent { searched, legacy } => {
            if options.config_path.is_some() {
                return Err(ConfigError::NotFound {
                    start: options.cwd,
                    searched,
                });
            }
            if options.targets.is_some() {
                return Err(ConfigError::NotFound {
                    start: options.cwd,
                    searched,
                });
            }
            let mut cli = TargetDraft::named("cli");
            apply_overrides(&mut cli, &env_overrides, &options.cwd)?;
            apply_overrides(&mut cli, &options.overrides, &options.cwd)?;
            if cli.is_complete() {
                (None, Vec::new(), vec![TargetDraft::named("cli")], searched)
            } else if let Some(path) = legacy {
                return Err(ConfigError::Legacy { path });
            } else {
                (None, Vec::new(), vec![TargetDraft::named("cli")], searched)
            }
        }
    };

    let available = drafts
        .iter()
        .map(|target| target.name.clone())
        .collect::<Vec<_>>();
    drafts = select_targets(drafts, options.targets.as_deref(), &available)?;
    if drafts.len() > 1
        && (env_overrides.has_target_values() || options.overrides.has_target_values())
    {
        return Err(ConfigError::AmbiguousOverride { available });
    }

    for draft in &mut drafts {
        apply_overrides(draft, &env_overrides, &options.cwd)?;
        apply_overrides(draft, &options.overrides, &options.cwd)?;
    }

    if config_path.is_none() && !drafts.iter().all(TargetDraft::is_complete) {
        return Err(ConfigError::NotFound {
            start: options.cwd,
            searched,
        });
    }

    let targets = drafts
        .into_iter()
        .map(|draft| draft.resolve(&options.env))
        .collect::<Result<Vec<_>, _>>()?;
    warnings.shrink_to_fit();
    Ok(ResolvedConfig {
        config_path,
        targets,
        warnings,
    })
}

/// Generate the JSON Schema accepted by v1 config files.
pub fn schema() -> Value {
    let mut schema = serde_json::to_value(schemars::schema_for!(RawConfig))
        .expect("RawConfig schema must be serializable");
    let object = schema
        .as_object_mut()
        .expect("schemars must generate an object schema");
    object.insert(
        "$schema".into(),
        Value::String("https://json-schema.org/draft/2020-12/schema".into()),
    );
    object.insert("$id".into(), Value::String(SCHEMA_ID.into()));
    schema
}

#[derive(Debug)]
enum Discovery {
    Found {
        path: PathBuf,
        warnings: Vec<String>,
    },
    Absent {
        searched: Vec<PathBuf>,
        legacy: Option<PathBuf>,
    },
}

fn discover(cwd: &Path, explicit: Option<&Path>) -> Result<Discovery, ConfigError> {
    if let Some(explicit) = explicit {
        let path = absolute_path(explicit, cwd);
        ensure_supported_extension(&path)?;
        if path.is_file() {
            return Ok(Discovery::Found {
                path,
                warnings: Vec::new(),
            });
        }
        return Ok(Discovery::Absent {
            searched: vec![path],
            legacy: None,
        });
    }

    let mut directory = cwd.to_path_buf();
    let mut searched = Vec::new();
    let mut legacy = None;
    loop {
        let found = CONFIG_NAMES
            .iter()
            .map(|name| directory.join(name))
            .collect::<Vec<_>>();
        let existing = found
            .iter()
            .filter(|path| path.is_file())
            .collect::<Vec<_>>();
        if let Some(path) = existing.first() {
            let warnings = if existing.len() > 1 {
                vec![format!(
                    "{} 同時有多種 gslm 設定檔，使用 {}",
                    directory.display(),
                    path.file_name().unwrap_or_default().to_string_lossy()
                )]
            } else {
                Vec::new()
            };
            return Ok(Discovery::Found {
                path: (*path).clone(),
                warnings,
            });
        }
        searched.extend(found);
        if legacy.is_none() {
            legacy = LEGACY_NAMES
                .iter()
                .map(|name| directory.join(name))
                .find(|path| path.is_file());
        }
        if directory.join(".git").exists() {
            break;
        }
        match directory.parent() {
            Some(parent) if parent != directory => directory = parent.to_path_buf(),
            _ => break,
        }
    }
    Ok(Discovery::Absent { searched, legacy })
}

fn ensure_supported_extension(path: &Path) -> Result<(), ConfigError> {
    match path.extension().and_then(OsStr::to_str) {
        Some("toml" | "json" | "jsonc") => Ok(()),
        _ => Err(ConfigError::Unsupported {
            path: path.to_path_buf(),
        }),
    }
}

fn read_config(path: &Path) -> Result<RawConfig, ConfigError> {
    ensure_supported_extension(path)?;
    let text = fs::read_to_string(path).map_err(|error| ConfigError::Parse {
        path: path.to_path_buf(),
        line: None,
        column: None,
        message: format!("無法讀取檔案：{error}"),
        source: Some(Box::new(error)),
    })?;
    let value = match path.extension().and_then(OsStr::to_str) {
        Some("toml") => toml::from_str::<Value>(&text).map_err(|error| {
            let (line, column) = error
                .span()
                .map(|span| line_column(&text, span.start))
                .map_or((None, None), |(line, column)| (Some(line), Some(column)));
            ConfigError::Parse {
                path: path.to_path_buf(),
                line,
                column,
                message: error.message().to_string(),
                source: Some(Box::new(error)),
            }
        })?,
        Some("json" | "jsonc") => json5::from_str::<Value>(&text).map_err(|error| {
            let (line, column) = match &error {
                json5::Error::Message { location, .. } => location
                    .as_ref()
                    .map(|location| (Some(location.line), Some(location.column)))
                    .unwrap_or((None, None)),
            };
            ConfigError::Parse {
                path: path.to_path_buf(),
                line,
                column,
                message: error.to_string(),
                source: Some(Box::new(error)),
            }
        })?,
        _ => unreachable!("extension checked above"),
    };
    validate_version(&value)?;
    validate_field_names(&value)?;
    serde_json::from_value(value).map_err(|error| ConfigError::Invalid {
        path: Some(path.to_path_buf()),
        field: "config".into(),
        message: error.to_string(),
    })
}

fn validate_version(value: &Value) -> Result<(), ConfigError> {
    let version = value
        .as_object()
        .and_then(|object| object.get("version"))
        .ok_or(ConfigError::MissingVersion)?;
    if version.as_u64() != Some(1) {
        return Err(ConfigError::UnsupportedVersion {
            found: version.to_string(),
        });
    }
    Ok(())
}

fn validate_field_names(value: &Value) -> Result<(), ConfigError> {
    let object = value.as_object().ok_or_else(|| ConfigError::Invalid {
        path: None,
        field: "config".into(),
        message: "設定檔根節點必須是物件".into(),
    })?;
    validate_object_fields(
        object,
        &[
            "version",
            "$schema",
            "sheet",
            "tab",
            "locales",
            "path",
            "format",
            "key_separator",
            "credentials",
            "targets",
        ],
        "",
    )?;
    if let Some(targets) = object.get("targets").and_then(Value::as_array) {
        for (index, target) in targets.iter().enumerate() {
            let target = target.as_object().ok_or_else(|| ConfigError::Invalid {
                path: None,
                field: format!("targets[{index}]"),
                message: "必須是物件".into(),
            })?;
            validate_object_fields(
                target,
                &[
                    "name",
                    "sheet",
                    "tab",
                    "locales",
                    "path",
                    "format",
                    "key_separator",
                    "credentials",
                ],
                &format!("targets[{index}]."),
            )?;
            validate_credentials_fields(target.get("credentials"), &format!("targets[{index}]."))?;
        }
    }
    validate_credentials_fields(object.get("credentials"), "")
}

fn validate_object_fields(
    object: &serde_json::Map<String, Value>,
    allowed: &[&str],
    prefix: &str,
) -> Result<(), ConfigError> {
    for field in object.keys() {
        if !allowed.contains(&field.as_str()) {
            let suggestion = legacy_suggestion(field).or_else(|| {
                nearest_field(field, allowed).map(|field| format!("是否要用 `{field}`？"))
            });
            return Err(ConfigError::UnknownField {
                field: format!("{prefix}{field}"),
                suggestion,
            });
        }
    }
    Ok(())
}

fn validate_credentials_fields(value: Option<&Value>, prefix: &str) -> Result<(), ConfigError> {
    let Some(value) = value else {
        return Ok(());
    };
    let object = value.as_object().ok_or_else(|| ConfigError::Invalid {
        path: None,
        field: format!("{prefix}credentials"),
        message: "必須是 { file = \"...\" } 或 { env = \"...\" }".into(),
    })?;
    for secret in ["private_key", "client_email"] {
        if object.contains_key(secret) {
            return Err(ConfigError::Invalid {
                path: None,
                field: format!("{prefix}credentials.{secret}"),
                message: "設定檔不可內嵌 Service Account 金鑰；請改用 credentials.file 或 credentials.env".into(),
            });
        }
    }
    if object.get("type").and_then(Value::as_str) == Some("service_account") {
        return Err(ConfigError::Invalid {
            path: None,
            field: format!("{prefix}credentials.type"),
            message:
                "設定檔不可內嵌 Service Account 金鑰；請改用 credentials.file 或 credentials.env"
                    .into(),
        });
    }
    validate_object_fields(object, &["file", "env"], &format!("{prefix}credentials."))?;
    if object.contains_key("file") && object.contains_key("env") {
        return Err(ConfigError::Invalid {
            path: None,
            field: format!("{prefix}credentials"),
            message: "file 與 env 只能擇一".into(),
        });
    }
    Ok(())
}

fn legacy_suggestion(field: &str) -> Option<String> {
    let replacement = match field {
        "sheetId" => "sheet",
        "sheetTitle" => "tab",
        "languages" => "locales",
        "directory" => "path",
        "type" => "format",
        _ => return None,
    };
    Some(format!(
        "舊欄位 `{field}` 已改為 `{replacement}`；請執行 `gslm migrate`"
    ))
}

fn nearest_field<'a>(field: &str, allowed: &'a [&str]) -> Option<&'a str> {
    allowed
        .iter()
        .copied()
        .map(|candidate| (candidate, levenshtein(field, candidate)))
        .min_by_key(|(_, distance)| *distance)
        .and_then(|(candidate, distance)| (distance <= 3).then_some(candidate))
}

fn levenshtein(left: &str, right: &str) -> usize {
    let mut row = (0..=right.chars().count()).collect::<Vec<_>>();
    for (i, a) in left.chars().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, b) in right.chars().enumerate() {
            let old = row[j + 1];
            row[j + 1] = if a == b {
                previous
            } else {
                1 + previous.min(row[j]).min(old)
            };
            previous = old;
        }
    }
    row[right.chars().count()]
}

fn line_column(input: &str, byte: usize) -> (usize, usize) {
    let before = &input[..byte.min(input.len())];
    let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = before
        .rsplit('\n')
        .next()
        .unwrap_or_default()
        .chars()
        .count()
        + 1;
    (line, column)
}

#[derive(Debug, Clone)]
struct TargetDraft {
    name: String,
    sheet: Option<String>,
    tab: Option<String>,
    locales: Option<Vec<String>>,
    path: Option<PathBuf>,
    format: Option<Format>,
    key_separator: Option<String>,
    credentials: Option<CredentialsDraft>,
}

#[derive(Debug, Clone)]
enum CredentialsDraft {
    Raw {
        value: RawCredentials,
        base: PathBuf,
    },
    Resolved(CredentialsSource),
}

impl TargetDraft {
    fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sheet: None,
            tab: None,
            locales: None,
            path: None,
            format: None,
            key_separator: None,
            credentials: None,
        }
    }

    fn is_complete(&self) -> bool {
        self.sheet.is_some() && self.tab.is_some() && self.locales.is_some() && self.path.is_some()
    }

    fn resolve(self, env: &BTreeMap<String, String>) -> Result<ResolvedTarget, ConfigError> {
        let name = self.name;
        let sheet = required(self.sheet, &name, "sheet")?;
        let tab = required(self.tab, &name, "tab")?;
        let locales = required(self.locales, &name, "locales")?;
        validate_locales(&locales, &name)?;
        let path = required(self.path, &name, "path")?;
        validate_path_template(&path, &name)?;
        let key_separator = self.key_separator.unwrap_or_else(|| ".".into());
        if key_separator.is_empty() {
            return Err(invalid_target(&name, "key_separator", "不可為空字串"));
        }
        let credentials = match self.credentials {
            Some(CredentialsDraft::Raw { value, base }) => credentials_from_raw(value, &base, env)?,
            Some(CredentialsDraft::Resolved(value)) => value,
            None => CredentialsSource::ApplicationDefault,
        };
        Ok(ResolvedTarget {
            name,
            sheet,
            tab,
            locales,
            path,
            format: self.format.unwrap_or(Format::Nest),
            key_separator,
            credentials,
        })
    }
}

fn required<T>(value: Option<T>, target: &str, field: &str) -> Result<T, ConfigError> {
    value.ok_or_else(|| invalid_target(target, field, "缺少必填欄位"))
}

fn invalid_target(target: &str, field: &str, message: &str) -> ConfigError {
    ConfigError::Invalid {
        path: None,
        field: format!("targets.{target}.{field}"),
        message: message.into(),
    }
}

fn validate_locales(locales: &[String], target: &str) -> Result<(), ConfigError> {
    if locales.is_empty() {
        return Err(invalid_target(target, "locales", "不可為空陣列"));
    }
    if locales.iter().any(String::is_empty) {
        return Err(invalid_target(target, "locales", "不可有空白 Locale"));
    }
    let mut seen = BTreeSet::new();
    if locales.iter().any(|locale| !seen.insert(locale)) {
        return Err(invalid_target(target, "locales", "不可有重複 Locale"));
    }
    Ok(())
}

fn validate_path_template(path: &Path, target: &str) -> Result<(), ConfigError> {
    let text = path.to_string_lossy();
    let mut has_locale = false;
    let mut rest = text.as_ref();
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else {
            return Err(invalid_target(target, "path", "佔位符必須以 } 結束"));
        };
        let placeholder = &after[..end];
        if placeholder != "locale" {
            return Err(invalid_target(
                target,
                "path",
                &format!("不支援 {{{placeholder}}}；目前只支援 {{locale}}"),
            ));
        }
        has_locale = true;
        rest = &after[end + 1..];
    }
    if rest.contains('}') {
        return Err(invalid_target(target, "path", "佔位符格式無效"));
    }
    if !has_locale {
        return Err(invalid_target(target, "path", "必須包含 {locale} 佔位符"));
    }
    Ok(())
}

fn drafts_from_raw(raw: RawConfig, config_dir: &Path) -> Result<Vec<TargetDraft>, ConfigError> {
    let _ = (raw.version, raw.schema.as_deref());
    let top = RawTarget::from_config(&raw);
    match raw.targets {
        None => Ok(vec![draft_from_target(
            "default".into(),
            &top,
            None,
            config_dir,
        )]),
        Some(targets) if targets.is_empty() => Err(ConfigError::Invalid {
            path: None,
            field: "targets".into(),
            message: "不可為空陣列；若只有一個 Target，請直接使用頂層欄位".into(),
        }),
        Some(targets) => {
            let mut names = BTreeSet::new();
            let mut drafts = Vec::with_capacity(targets.len());
            for target in targets {
                let name = target.name.clone().ok_or_else(|| ConfigError::Invalid {
                    path: None,
                    field: "targets.name".into(),
                    message: "每個 Target 都必須有唯一 name".into(),
                })?;
                if !names.insert(name.clone()) {
                    return Err(ConfigError::Invalid {
                        path: None,
                        field: "targets.name".into(),
                        message: format!("Target name `{name}` 重複"),
                    });
                }
                drafts.push(draft_from_target(name, &top, Some(&target), config_dir));
            }
            Ok(drafts)
        }
    }
}

fn draft_from_target(
    name: String,
    top: &RawTarget,
    target: Option<&RawTarget>,
    config_dir: &Path,
) -> TargetDraft {
    let value = |target: Option<&RawTarget>, get: fn(&RawTarget) -> Option<String>| {
        target.and_then(get).or_else(|| get(top))
    };
    let locales = target
        .and_then(|target| target.locales.clone())
        .or_else(|| top.locales.clone());
    let path = value(target, |target| target.path.clone())
        .map(|path| absolute_path(Path::new(&path), config_dir));
    let format = target
        .and_then(|target| target.format)
        .or(top.format)
        .map(Into::into);
    let key_separator = value(target, |target| target.key_separator.clone());
    let raw_credentials = target
        .and_then(|target| target.credentials.clone())
        .or_else(|| top.credentials.clone());
    TargetDraft {
        name,
        sheet: value(target, |target| target.sheet.clone()),
        tab: value(target, |target| target.tab.clone()),
        locales,
        path,
        format,
        key_separator,
        credentials: raw_credentials.map(|value| CredentialsDraft::Raw {
            value,
            base: config_dir.to_path_buf(),
        }),
    }
}

fn select_targets(
    drafts: Vec<TargetDraft>,
    requested: Option<&[String]>,
    available: &[String],
) -> Result<Vec<TargetDraft>, ConfigError> {
    let Some(requested) = requested else {
        return Ok(drafts);
    };
    let mut selected = Vec::with_capacity(requested.len());
    let mut selected_names = BTreeSet::new();
    for name in requested {
        if !selected_names.insert(name) {
            continue;
        }
        let target = drafts
            .iter()
            .find(|target| target.name == *name)
            .cloned()
            .ok_or_else(|| ConfigError::UnknownTarget {
                name: name.clone(),
                available: available.to_vec(),
            })?;
        selected.push(target);
    }
    Ok(selected)
}

fn environment_overrides(
    env: &BTreeMap<String, String>,
    cwd: &Path,
) -> Result<Overrides, ConfigError> {
    let value = |name: &str| env.get(name).filter(|value| !value.is_empty()).cloned();
    let credentials = value("GSLM_CREDENTIALS");
    let credentials_json = value("GSLM_CREDENTIALS_JSON");
    if credentials.is_some() && credentials_json.is_some() {
        return Err(ConfigError::Invalid {
            path: None,
            field: "credentials".into(),
            message: "GSLM_CREDENTIALS 與 GSLM_CREDENTIALS_JSON 只能擇一".into(),
        });
    }
    let format = match value("GSLM_FORMAT") {
        Some(value) => Some(parse_format(&value, "GSLM_FORMAT")?),
        None => None,
    };
    let locales = value("GSLM_LOCALES").map(|value| {
        value
            .split(',')
            .map(|locale| locale.trim().to_string())
            .collect()
    });
    let mut result = Overrides {
        sheet: value("GSLM_SHEET"),
        tab: value("GSLM_TAB"),
        locales,
        path: value("GSLM_PATH"),
        format,
        key_separator: value("GSLM_KEY_SEPARATOR"),
        credentials,
        credentials_json,
    };
    if let Some(path) = result.path.take() {
        result.path = Some(
            absolute_path(Path::new(&path), cwd)
                .to_string_lossy()
                .into_owned(),
        );
    }
    if let Some(file) = result.credentials.take() {
        result.credentials = Some(
            absolute_path(Path::new(&file), cwd)
                .to_string_lossy()
                .into_owned(),
        );
    }
    Ok(result)
}

fn apply_overrides(
    draft: &mut TargetDraft,
    overrides: &Overrides,
    cwd: &Path,
) -> Result<(), ConfigError> {
    overrides.validate_credentials()?;
    if let Some(value) = &overrides.sheet {
        draft.sheet = Some(value.clone());
    }
    if let Some(value) = &overrides.tab {
        draft.tab = Some(value.clone());
    }
    if let Some(value) = &overrides.locales {
        draft.locales = Some(value.clone());
    }
    if let Some(value) = &overrides.path {
        draft.path = Some(absolute_path(Path::new(value), cwd));
    }
    if let Some(value) = overrides.format {
        draft.format = Some(value);
    }
    if let Some(value) = &overrides.key_separator {
        draft.key_separator = Some(value.clone());
    }
    if let Some(value) = &overrides.credentials {
        draft.credentials = Some(CredentialsDraft::Resolved(CredentialsSource::File(
            absolute_path(Path::new(value), cwd),
        )));
    }
    if let Some(value) = &overrides.credentials_json {
        draft.credentials = Some(CredentialsDraft::Resolved(CredentialsSource::Json {
            env_name: "GSLM_CREDENTIALS_JSON".into(),
            value: value.clone(),
        }));
    }
    Ok(())
}

fn parse_format(value: &str, field: &str) -> Result<Format, ConfigError> {
    match value {
        "nest" => Ok(Format::Nest),
        "flat" => Ok(Format::Flat),
        _ => Err(ConfigError::Invalid {
            path: None,
            field: field.into(),
            message: "只能是 `nest` 或 `flat`".into(),
        }),
    }
}

fn credentials_from_raw(
    raw: RawCredentials,
    base: &Path,
    env: &BTreeMap<String, String>,
) -> Result<CredentialsSource, ConfigError> {
    match raw {
        RawCredentials::File(RawCredentialsFile { file }) => Ok(CredentialsSource::File(
            absolute_path(Path::new(&file), base),
        )),
        RawCredentials::Env(RawCredentialsEnv { env: name }) => {
            let value = env
                .get(&name)
                .filter(|value| !value.is_empty())
                .cloned()
                .ok_or_else(|| ConfigError::MissingEnv { name: name.clone() })?;
            Ok(CredentialsSource::Json {
                env_name: name,
                value,
            })
        }
    }
}

fn load_dotenv(cwd: &Path, env: &mut BTreeMap<String, String>) -> Result<(), ConfigError> {
    let path = cwd.join(".env");
    if !path.is_file() {
        return Ok(());
    }
    let values = dotenvy::from_path_iter(&path).map_err(|_| ConfigError::Parse {
        path: path.clone(),
        line: None,
        column: None,
        message: "無法解析 .env 檔".into(),
        source: None,
    })?;
    for value in values {
        let (key, value) = value.map_err(|_| ConfigError::Parse {
            path: path.clone(),
            line: None,
            column: None,
            message: "無法解析 .env 檔".into(),
            source: None,
        })?;
        match env.entry(key) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(value);
            }
            std::collections::btree_map::Entry::Occupied(mut entry) if entry.get().is_empty() => {
                entry.insert(value);
            }
            std::collections::btree_map::Entry::Occupied(_) => {}
        }
    }
    Ok(())
}

fn absolute_path(path: &Path, base: &Path) -> PathBuf {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    };
    normalize_path(&path)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::RootDir | Component::Prefix(_) | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    normalized
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    #[schemars(range(min = 1, max = 1))]
    version: u32,
    #[serde(rename = "$schema")]
    schema: Option<String>,
    sheet: Option<String>,
    tab: Option<String>,
    #[schemars(length(min = 1), inner(length(min = 1)))]
    locales: Option<Vec<String>>,
    #[schemars(pattern(r".*\{locale\}.*"))]
    path: Option<String>,
    format: Option<RawFormat>,
    #[schemars(length(min = 1))]
    key_separator: Option<String>,
    credentials: Option<RawCredentials>,
    targets: Option<Vec<RawTarget>>,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RawTarget {
    #[schemars(required)]
    name: Option<String>,
    sheet: Option<String>,
    tab: Option<String>,
    #[schemars(length(min = 1), inner(length(min = 1)))]
    locales: Option<Vec<String>>,
    #[schemars(pattern(r".*\{locale\}.*"))]
    path: Option<String>,
    format: Option<RawFormat>,
    #[schemars(length(min = 1))]
    key_separator: Option<String>,
    credentials: Option<RawCredentials>,
}

impl RawTarget {
    fn from_config(config: &RawConfig) -> Self {
        Self {
            name: None,
            sheet: config.sheet.clone(),
            tab: config.tab.clone(),
            locales: config.locales.clone(),
            path: config.path.clone(),
            format: config.format,
            key_separator: config.key_separator.clone(),
            credentials: config.credentials.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum RawFormat {
    Nest,
    Flat,
}

impl From<RawFormat> for Format {
    fn from(value: RawFormat) -> Self {
        match value {
            RawFormat::Nest => Self::Nest,
            RawFormat::Flat => Self::Flat,
        }
    }
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(untagged)]
enum RawCredentials {
    File(RawCredentialsFile),
    Env(RawCredentialsEnv),
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RawCredentialsFile {
    file: String,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RawCredentialsEnv {
    env: String,
}
