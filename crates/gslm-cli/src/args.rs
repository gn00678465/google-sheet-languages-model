use crate::ColorChoice;
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(
    name = "gslm",
    version,
    about = "同步 Google Sheets 與本地 i18n Catalog"
)]
pub(crate) struct Cli {
    #[command(flatten)]
    pub common: Common,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Args)]
pub(crate) struct Common {
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
    #[arg(short = 't', long, global = true, value_delimiter = ',')]
    pub target: Vec<String>,
    #[arg(short = 'q', long, global = true)]
    pub quiet: bool,
    #[arg(short = 'v', long, global = true)]
    pub verbose: bool,
    #[arg(long, global = true, value_enum)]
    pub color: Option<ColorArg>,
    #[arg(long, global = true)]
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
    Pull(SyncArgs),
    Push(PushArgs),
    Init(InitArgs),
    Schema,
    /// `bin/gslm.js` intercepts this so migration works without the binding.
    Migrate,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct SyncArgs {
    #[command(flatten)]
    pub fields: FieldOverrides,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct PushArgs {
    #[command(flatten)]
    pub fields: FieldOverrides,
    #[arg(long)]
    pub dry_run: bool,
    #[arg(long)]
    pub force: bool,
    #[arg(long)]
    pub strict: bool,
}

#[derive(Debug, Clone, Args, Default)]
pub(crate) struct FieldOverrides {
    #[arg(long)]
    pub sheet: Option<String>,
    #[arg(long)]
    pub tab: Option<String>,
    #[arg(long, value_delimiter = ',')]
    pub locales: Option<Vec<String>>,
    #[arg(long)]
    pub path: Option<String>,
    #[arg(long, value_enum)]
    pub format: Option<FormatArg>,
    #[arg(long)]
    pub key_separator: Option<String>,
    #[arg(long)]
    pub credentials: Option<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum FormatArg {
    Nest,
    Flat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct InitArgs {
    #[arg(long, value_enum, default_value_t = InitFormat::Toml)]
    pub format: InitFormat,
    #[arg(long)]
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
