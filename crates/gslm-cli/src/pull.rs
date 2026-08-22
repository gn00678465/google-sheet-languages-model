use crate::catalog_fs::{WriteOutcome, read_catalog, render_path, write_catalog};
use crate::{CliError, SheetsOverride, build_client};
use gslm_config::ResolvedTarget;
use gslm_sheets::SheetsClient;
use std::path::PathBuf;

/// Options shared by CLI and high-level SDK pull calls.
#[derive(Debug, Clone, Default)]
pub struct PullOptions {
    pub dry_run: bool,
    pub force: bool,
    pub sheets: SheetsOverride,
}

/// One local Catalog affected by a pull.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSummary {
    pub locale: String,
    pub path: PathBuf,
    pub keys: usize,
    pub outcome: Option<WriteOutcome>,
}

/// Data-only result suitable for N-API conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullSummary {
    pub target: String,
    pub files: Vec<FileSummary>,
    pub created: usize,
    pub updated: usize,
    pub unchanged: usize,
}

/// Pull a single already-resolved Target, building a Sheets client as needed.
pub async fn pull(target: &ResolvedTarget, options: PullOptions) -> Result<PullSummary, CliError> {
    let client = build_client(&target.credentials, &options.sheets).await?;
    pull_with_client(target, options, &client).await
}

pub(crate) async fn pull_with_client(
    target: &ResolvedTarget,
    options: PullOptions,
    client: &SheetsClient,
) -> Result<PullSummary, CliError> {
    let table = client.read_tab(&target.sheet, &target.tab).await?;
    let model = gslm_core::Model::from_table(&table, target.locales.clone())?;

    if model.ordered_keys().is_empty() && !options.force {
        let mut local_keys = 0;
        for locale in &target.locales {
            let path = render_path(&target.path, locale);
            match read_catalog(&path, &target.key_separator) {
                Ok(Some(read)) => local_keys += read.catalog.len(),
                Ok(None) => {}
                Err(_) if path.exists() => local_keys += 1,
                Err(error) => return Err(error),
            }
        }
        if local_keys > 0 {
            return Err(CliError::PullEmptySheet { local_keys });
        }
    }

    let mut summary = PullSummary {
        target: target.name.clone(),
        files: Vec::with_capacity(target.locales.len()),
        created: 0,
        updated: 0,
        unchanged: 0,
    };
    for locale in &target.locales {
        let path = render_path(&target.path, locale);
        let catalog = model
            .catalog(locale)
            .expect("requested locale exists in model");
        let outcome = if options.dry_run {
            None
        } else {
            Some(write_catalog(
                &path,
                catalog,
                target.format,
                &target.key_separator,
            )?)
        };
        match outcome {
            Some(WriteOutcome::Created) => summary.created += 1,
            Some(WriteOutcome::Updated) => summary.updated += 1,
            Some(WriteOutcome::Unchanged) => summary.unchanged += 1,
            None => {}
        }
        summary.files.push(FileSummary {
            locale: locale.clone(),
            path,
            keys: catalog.len(),
            outcome,
        });
    }
    Ok(summary)
}
