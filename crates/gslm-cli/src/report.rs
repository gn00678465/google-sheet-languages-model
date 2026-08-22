use crate::{ColorChoice, PullSummary, PushSummary};
use gslm_config::{CredentialsSource, ResolvedTarget};
use std::io::Write;

pub(crate) struct Reporter<'a> {
    stdout: &'a mut (dyn Write + Send),
    stderr: &'a mut (dyn Write + Send),
    quiet: bool,
    verbose: bool,
    color: bool,
}

impl<'a> Reporter<'a> {
    pub(crate) fn new(
        stdout: &'a mut (dyn Write + Send),
        stderr: &'a mut (dyn Write + Send),
        quiet: bool,
        verbose: bool,
        choice: Option<ColorChoice>,
        is_tty: Option<bool>,
        no_color: bool,
    ) -> Self {
        let color = match choice.unwrap_or(ColorChoice::Auto) {
            ColorChoice::Always => true,
            ColorChoice::Never => false,
            ColorChoice::Auto => is_tty.unwrap_or(false) && !no_color,
        };
        Self {
            stdout,
            stderr,
            quiet,
            verbose,
            color,
        }
    }

    pub(crate) fn warning(&mut self, message: &str) {
        let prefix = if self.color {
            "\u{1b}[33m警告：\u{1b}[0m"
        } else {
            "警告："
        };
        let _ = writeln!(self.stderr, "{prefix}{message}");
    }

    pub(crate) fn target(&mut self, target: &ResolvedTarget) {
        if self.quiet {
            return;
        }
        let credentials = match &target.credentials {
            CredentialsSource::File(_) => "檔案",
            CredentialsSource::Json { .. } => "環境變數 JSON",
            CredentialsSource::ApplicationDefault => "Application Default Credentials",
        };
        let _ = writeln!(
            self.stderr,
            "目標 {}：Sheet={}、Tab={}、語言={}、路徑={}、格式={:?}、憑證={credentials}",
            target.name,
            target.sheet,
            target.tab,
            target.locales.join(","),
            target.path.display(),
            target.format,
        );
    }

    pub(crate) fn request_details(&mut self, target: &ResolvedTarget, is_pull: bool) {
        if !self.verbose {
            return;
        }
        let action = if is_pull { "讀取" } else { "寫入" };
        let _ = writeln!(
            self.stderr,
            "詳細：{action} Sheet {} 的 Tab {}",
            target.sheet, target.tab
        );
        for locale in &target.locales {
            let path = crate::catalog_fs::render_path(&target.path, locale);
            let _ = writeln!(self.stderr, "  Catalog：{}", path.display());
        }
    }

    pub(crate) fn pull_summary(&mut self, summary: &PullSummary, dry_run: bool) {
        let writer = if dry_run {
            &mut self.stdout
        } else {
            &mut self.stderr
        };
        if dry_run {
            let _ = writeln!(
                writer,
                "目標 {}：預覽模式會寫入 {} 個檔案",
                summary.target,
                summary.files.len()
            );
        } else if !self.quiet {
            let _ = writeln!(
                writer,
                "目標 {}：新增 {}、變更 {}、未變動 {}",
                summary.target, summary.created, summary.updated, summary.unchanged
            );
        }
        if !self.quiet {
            for file in &summary.files {
                let _ = writeln!(writer, "  {}：{} 個 key", file.path.display(), file.keys);
            }
        }
    }

    pub(crate) fn push_summary(&mut self, summary: &PushSummary, dry_run: bool) {
        let writer = if dry_run {
            &mut self.stdout
        } else {
            &mut self.stderr
        };
        if dry_run {
            let _ = writeln!(
                writer,
                "目標 {}：預覽模式會寫入 {} 列、{} 欄",
                summary.target, summary.rows, summary.columns
            );
            if !summary.orphan_keys.is_empty() {
                let _ = writeln!(writer, "  孤立 key：{}", summary.orphan_keys.join(", "));
            }
        } else if !self.quiet {
            let _ = writeln!(
                writer,
                "目標 {}：已寫入 {} 列、{} 欄",
                summary.target, summary.rows, summary.columns
            );
        }
        if !self.quiet {
            for (locale, keys) in &summary.locale_keys {
                let _ = writeln!(writer, "  {locale}：{keys} 個 key");
            }
        }
    }
}
