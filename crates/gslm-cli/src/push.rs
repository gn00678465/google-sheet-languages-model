use crate::catalog_fs::{read_catalog, render_path, shape_mismatch};
use crate::{CliError, SheetsOverride, build_client};
use gslm_config::ResolvedTarget;
use gslm_core::Model;
use gslm_sheets::SheetsClient;

/// Options shared by CLI and high-level SDK push calls.
#[derive(Debug, Clone, Default)]
pub struct PushOptions {
    pub dry_run: bool,
    pub force: bool,
    pub strict: bool,
    pub sheets: SheetsOverride,
}

/// Data-only result of writing a Tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushSummary {
    pub target: String,
    pub rows: usize,
    pub columns: usize,
    pub locale_keys: Vec<(String, usize)>,
    pub orphan_keys: Vec<String>,
    pub warnings: Vec<String>,
}

/// Push a single already-resolved Target, building a Sheets client as needed.
pub async fn push(target: &ResolvedTarget, options: PushOptions) -> Result<PushSummary, CliError> {
    let client = build_client(&target.credentials, &options.sheets).await?;
    push_with_client(target, options, &client).await
}

pub(crate) async fn push_with_client(
    target: &ResolvedTarget,
    options: PushOptions,
    client: &SheetsClient,
) -> Result<PushSummary, CliError> {
    let mut model = Model::new(target.locales.clone());
    let mut warnings = Vec::new();
    let mut shape_errors = Vec::new();
    for locale in &target.locales {
        let path = render_path(&target.path, locale);
        let Some(read) = read_catalog(&path, &target.key_separator)? else {
            warnings.push(format!(
                "找不到 {locale} 的 Catalog：{}；視為空白",
                path.display()
            ));
            continue;
        };
        if shape_mismatch(&read, target.format) {
            let message = format!("{} 的實際形狀與設定 format 不符", path.display());
            warnings.push(message.clone());
            shape_errors.push(message);
        }
        model.set_catalog(locale, read.catalog)?;
    }
    let all_empty = model.catalogs().values().all(gslm_core::Catalog::is_empty);
    if all_empty && !options.force {
        return Err(CliError::PushEmptyLocal);
    }
    let orphan_keys = model.orphan_keys();
    if !orphan_keys.is_empty() {
        warnings.push(format!(
            "發現只存在非來源 Locale 的孤立 key：{}",
            orphan_keys.join(", ")
        ));
    }
    if options.strict && (!shape_errors.is_empty() || !orphan_keys.is_empty()) {
        let mut reasons = shape_errors;
        if !orphan_keys.is_empty() {
            reasons.push(format!("孤立 key：{}", orphan_keys.join(", ")));
        }
        return Err(CliError::PushStrict { reasons });
    }
    let table = model.to_table();
    if !options.dry_run {
        client.write_tab(&target.sheet, &target.tab, &table).await?;
    }
    let locale_keys = target
        .locales
        .iter()
        .map(|locale| {
            (
                locale.clone(),
                model.catalog(locale).expect("registered locale").len(),
            )
        })
        .collect();
    Ok(PushSummary {
        target: target.name.clone(),
        rows: table.len(),
        columns: table.first().map_or(0, Vec::len),
        locale_keys,
        orphan_keys,
        warnings,
    })
}
