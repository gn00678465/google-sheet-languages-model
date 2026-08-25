use crate::{ColorChoice, PullSummary, PushSummary, credential_details};
use gslm_config::ResolvedTarget;
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
        let credentials = credential_details(&target.credentials).label;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FileSummary, WriteOutcome};
    use gslm_config::{CredentialsSource, ResolvedTarget};
    use gslm_core::Format;
    use std::path::PathBuf;

    fn target() -> ResolvedTarget {
        ResolvedTarget {
            name: "web".into(),
            sheet: "sheet".into(),
            tab: "Main".into(),
            locales: vec!["en".into(), "zh-TW".into()],
            path: PathBuf::from("/project/locales/{locale}.json"),
            format: Format::Nest,
            key_separator: ".".into(),
            credentials: CredentialsSource::ApplicationDefault,
        }
    }

    #[test]
    fn reporter_routes_dry_run_details_and_colored_warnings_to_expected_streams() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        {
            let mut reporter = Reporter::new(
                &mut stdout,
                &mut stderr,
                false,
                true,
                Some(ColorChoice::Always),
                Some(false),
                true,
            );
            let target = target();
            reporter.warning("needs attention");
            reporter.target(&target);
            reporter.request_details(&target, true);
            reporter.pull_summary(
                &PullSummary {
                    target: "web".into(),
                    files: vec![FileSummary {
                        locale: "en".into(),
                        path: PathBuf::from("/project/locales/en.json"),
                        keys: 2,
                        outcome: Some(WriteOutcome::Created),
                    }],
                    created: 1,
                    updated: 0,
                    unchanged: 0,
                },
                true,
            );
            reporter.push_summary(
                &PushSummary {
                    target: "web".into(),
                    rows: 3,
                    columns: 3,
                    locale_keys: vec![("en".into(), 2), ("zh-TW".into(), 1)],
                    orphan_keys: vec!["orphan".into()],
                    warnings: Vec::new(),
                },
                true,
            );
        }

        let stdout = String::from_utf8(stdout).unwrap();
        let stderr = String::from_utf8(stderr).unwrap();
        assert!(stdout.contains("預覽模式會寫入 1 個檔案"));
        assert!(stdout.contains("預覽模式會寫入 3 列、3 欄"));
        assert!(stdout.contains("孤立 key：orphan"));
        assert!(stdout.contains("en：2 個 key"));
        assert!(stderr.contains("\u{1b}[33m警告：\u{1b}[0mneeds attention"));
        assert!(stderr.contains("目標 web：Sheet=sheet"));
        assert!(stderr.contains("詳細：讀取 Sheet sheet 的 Tab Main"));
        assert!(stderr.contains("Catalog：/project/locales/en.json"));
    }

    #[test]
    fn quiet_reporter_suppresses_non_warning_progress() {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        {
            let mut reporter = Reporter::new(
                &mut stdout,
                &mut stderr,
                true,
                false,
                Some(ColorChoice::Auto),
                Some(true),
                false,
            );
            reporter.target(&target());
            reporter.pull_summary(
                &PullSummary {
                    target: "web".into(),
                    files: Vec::new(),
                    created: 0,
                    updated: 0,
                    unchanged: 0,
                },
                false,
            );
            reporter.push_summary(
                &PushSummary {
                    target: "web".into(),
                    rows: 1,
                    columns: 1,
                    locale_keys: Vec::new(),
                    orphan_keys: Vec::new(),
                    warnings: Vec::new(),
                },
                false,
            );
            reporter.warning("still visible");
        }

        assert!(stdout.is_empty());
        assert_eq!(
            String::from_utf8(stderr).unwrap(),
            "\u{1b}[33m警告：\u{1b}[0mstill visible\n"
        );
    }
}
