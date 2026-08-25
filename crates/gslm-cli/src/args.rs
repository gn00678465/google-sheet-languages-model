use crate::ColorChoice;
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "gslm",
    version,
    about = "同步 Google Sheets 與本地 i18n Catalog",
    long_about = "同步 Google Sheets 與本地 i18n Catalog。使用 pull 下載翻譯，使用 push 上傳翻譯。",
    subcommand_help_heading = "指令",
    next_help_heading = "選項",
    help_template = "{before-help}{about-with-newline}\n用法：{usage}\n\n{all-args}{after-help}",
    disable_help_flag = true,
    disable_version_flag = true,
    disable_help_subcommand = true
)]
pub(crate) struct Cli {
    #[command(flatten)]
    pub common: Common,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Args)]
pub(crate) struct Common {
    #[arg(long, global = true, help = "設定檔路徑")]
    pub config: Option<PathBuf>,
    #[arg(
        short = 't',
        long,
        global = true,
        value_delimiter = ',',
        help = "只執行指定 Target（可重複或以逗號分隔）"
    )]
    pub target: Vec<String>,
    #[arg(short = 'q', long, global = true, help = "隱藏一般進度訊息")]
    pub quiet: bool,
    #[arg(short = 'v', long, global = true, help = "顯示請求與 Catalog 路徑細節")]
    pub verbose: bool,
    #[arg(
        long,
        global = true,
        value_enum,
        hide_possible_values = true,
        help = "設定診斷訊息顏色（auto、always、never）"
    )]
    pub color: Option<ColorArg>,
    #[arg(long, global = true, help = "不要載入 .env")]
    pub no_dotenv: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum ColorArg {
    Auto,
    Always,
    Never,
}

impl From<ColorArg> for ColorChoice {
    fn from(value: ColorArg) -> Self {
        match value {
            ColorArg::Auto => Self::Auto,
            ColorArg::Always => Self::Always,
            ColorArg::Never => Self::Never,
        }
    }
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    #[command(
        about = "從 Google Sheets 下載 Catalog",
        next_help_heading = "選項",
        help_template = "{before-help}{about-with-newline}\n用法：{usage}\n\n{all-args}{after-help}"
    )]
    Pull(SyncArgs),
    #[command(
        about = "將本地 Catalog 上傳到 Google Sheets",
        next_help_heading = "選項",
        help_template = "{before-help}{about-with-newline}\n用法：{usage}\n\n{all-args}{after-help}"
    )]
    Push(PushArgs),
    #[command(
        about = "建立 gslm 設定檔範本",
        next_help_heading = "選項",
        help_template = "{before-help}{about-with-newline}\n用法：{usage}\n\n{all-args}{after-help}"
    )]
    Init(InitArgs),
    #[command(
        about = "將設定 JSON Schema 輸出到標準輸出",
        next_help_heading = "選項",
        help_template = "{before-help}{about-with-newline}\n用法：{usage}\n\n{all-args}{after-help}"
    )]
    Schema,
    #[command(
        about = "遷移舊版可執行設定檔（由 JS 入口處理）",
        next_help_heading = "選項",
        help_template = "{before-help}{about-with-newline}\n用法：{usage}\n\n{all-args}{after-help}"
    )]
    Migrate,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct SyncArgs {
    #[command(flatten)]
    pub fields: FieldOverrides,
    #[arg(long, help = "只顯示會寫入的內容，不修改檔案")]
    pub dry_run: bool,
    #[arg(long, help = "略過空 Sheet 保護")]
    pub force: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct PushArgs {
    #[command(flatten)]
    pub fields: FieldOverrides,
    #[arg(long, help = "只顯示會寫入的內容，不修改檔案或 Sheet")]
    pub dry_run: bool,
    #[arg(long, help = "允許將全空本地 Catalog 寫入 Sheet")]
    pub force: bool,
    #[arg(long, help = "將孤立 key 與格式漂移視為錯誤")]
    pub strict: bool,
}

#[derive(Debug, Clone, Args, Default)]
pub(crate) struct FieldOverrides {
    #[arg(long, help = "覆寫 Google Sheet ID")]
    pub sheet: Option<String>,
    #[arg(long, help = "覆寫 Sheet 分頁名稱")]
    pub tab: Option<String>,
    #[arg(long, value_delimiter = ',', help = "覆寫 Locale 清單")]
    pub locales: Option<Vec<String>>,
    #[arg(long, help = "覆寫 Catalog 路徑樣板")]
    pub path: Option<String>,
    #[arg(
        long,
        value_enum,
        hide_possible_values = true,
        help = "覆寫 Catalog 格式（nest、flat）"
    )]
    pub format: Option<FormatArg>,
    #[arg(long, help = "覆寫巢狀 key 分隔符")]
    pub key_separator: Option<String>,
    #[arg(long, help = "覆寫服務帳號憑證檔路徑")]
    pub credentials: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum FormatArg {
    Nest,
    Flat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct InitArgs {
    #[arg(
        long,
        value_enum,
        default_value_t = InitFormat::Toml,
        hide_possible_values = true,
        help = "範本格式（toml、jsonc）"
    )]
    pub format: InitFormat,
    #[arg(long, help = "覆寫既有設定檔")]
    pub force: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum InitFormat {
    Toml,
    Jsonc,
}

/// Command data with global options copied in, making command execution
/// independent from clap's representation.
#[derive(Debug, Clone)]
pub(crate) struct SyncCommand {
    pub config: Option<PathBuf>,
    pub targets: Vec<String>,
    pub no_dotenv: bool,
    pub overrides: FieldOverrides,
    pub dry_run: bool,
    pub force: bool,
    pub strict: bool,
    pub quiet: bool,
    pub verbose: bool,
    pub color: Option<ColorChoice>,
}

impl Cli {
    pub fn sync_command(&self, args: SyncArgs) -> SyncCommand {
        SyncCommand {
            config: self.common.config.clone(),
            targets: self.common.target.clone(),
            no_dotenv: self.common.no_dotenv,
            overrides: args.fields,
            dry_run: args.dry_run,
            force: args.force,
            strict: false,
            quiet: self.common.quiet,
            verbose: self.common.verbose,
            color: self.common.color.map(Into::into),
        }
    }

    pub fn push_command(&self, args: PushArgs) -> SyncCommand {
        SyncCommand {
            config: self.common.config.clone(),
            targets: self.common.target.clone(),
            no_dotenv: self.common.no_dotenv,
            overrides: args.fields,
            dry_run: args.dry_run,
            force: args.force,
            strict: args.strict,
            quiet: self.common.quiet,
            verbose: self.common.verbose,
            color: self.common.color.map(Into::into),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_arguments_preserve_every_explicit_choice() {
        assert_eq!(ColorChoice::from(ColorArg::Auto), ColorChoice::Auto);
        assert_eq!(ColorChoice::from(ColorArg::Always), ColorChoice::Always);
        assert_eq!(ColorChoice::from(ColorArg::Never), ColorChoice::Never);
    }
}
