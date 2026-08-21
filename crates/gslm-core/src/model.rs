use crate::catalog::Catalog;
use crate::error::ConversionError;
use indexmap::IndexMap;

/// A language/region code such as `en` or `zh-TW`. Not validated here.
pub type Locale = String;

/// The shape exchanged with a Tab: first row is the header
/// (`key` + locales), every following row is one key.
pub type Table = Vec<Vec<String>>;

/// Header text of the key column when writing. When reading, the first column
/// is always the key column regardless of its header text.
pub const KEY_HEADER: &str = "key";

/// Ordered set of locales, each with its own Catalog. `locales[0]` is the
/// Source locale: its key order drives the Tab's row order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    locales: Vec<Locale>,
    catalogs: IndexMap<Locale, Catalog>,
}

impl Model {
    /// Create a Model with an empty Catalog per locale. Duplicate locales are
    /// collapsed to their first occurrence.
    pub fn new<I, S>(locales: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<Locale>,
    {
        let mut catalogs: IndexMap<Locale, Catalog> = IndexMap::new();
        for locale in locales {
            catalogs.entry(locale.into()).or_default();
        }
        let locales = catalogs.keys().cloned().collect();
        Self { locales, catalogs }
    }

    pub fn locales(&self) -> &[Locale] {
        &self.locales
    }

    pub fn source_locale(&self) -> Option<&Locale> {
        self.locales.first()
    }

    pub fn catalog(&self, locale: &str) -> Option<&Catalog> {
        self.catalogs.get(locale)
    }

    pub fn catalogs(&self) -> &IndexMap<Locale, Catalog> {
        &self.catalogs
    }

    /// Replace the Catalog of a known locale.
    pub fn set_catalog(&mut self, locale: &str, catalog: Catalog) -> Result<(), ConversionError> {
        match self.catalogs.get_mut(locale) {
            Some(slot) => {
                *slot = catalog;
                Ok(())
            }
            None => Err(ConversionError::UnknownLocale(locale.to_string())),
        }
    }

    /// Keys present in some non-Source locale but absent from the Source
    /// locale, in the order they will be appended to the Tab.
    pub fn orphan_keys(&self) -> Vec<String> {
        let Some(source) = self.source_locale() else {
            return Vec::new();
        };
        let source_catalog = &self.catalogs[source];
        let mut seen: IndexMap<&str, ()> = IndexMap::new();
        for (locale, catalog) in &self.catalogs {
            if locale == source {
                continue;
            }
            for key in catalog.keys() {
                if !source_catalog.contains_key(key) {
                    seen.entry(key).or_insert(());
                }
            }
        }
        seen.keys().map(|k| k.to_string()).collect()
    }

    /// All keys in Tab row order: Source keys first, then orphan keys.
    pub fn ordered_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = match self.source_locale() {
            Some(source) => self.catalogs[source].keys().map(str::to_string).collect(),
            None => Vec::new(),
        };
        keys.extend(self.orphan_keys());
        keys
    }

    /// Build the Tab rows for push. Missing translations and empty strings
    /// both become empty cells.
    pub fn to_table(&self) -> Table {
        let mut header = Vec::with_capacity(self.locales.len() + 1);
        header.push(KEY_HEADER.to_string());
        header.extend(self.locales.iter().cloned());

        let mut rows = vec![header];
        for key in self.ordered_keys() {
            let mut row = Vec::with_capacity(self.locales.len() + 1);
            row.push(key.clone());
            for locale in &self.locales {
                row.push(self.catalogs[locale].get(&key).unwrap_or("").to_string());
            }
            rows.push(row);
        }
        rows
    }

    /// Parse Tab rows for pull. Columns are matched to `locales` by header
    /// text; the first column is the key column. Empty cells and short rows
    /// mean "missing"; rows with an empty key are skipped; duplicate keys and
    /// missing locales are errors.
    pub fn from_table<I, S>(rows: &Table, locales: I) -> Result<Self, ConversionError>
    where
        I: IntoIterator<Item = S>,
        S: Into<Locale>,
    {
        let Some((header, body)) = rows.split_first() else {
            return Err(ConversionError::EmptySheet);
        };
        let mut model = Model::new(locales);

        // Column index per requested locale, matched by header text
        // (the first column is the key column whatever its header says).
        let available: Vec<String> = header.iter().skip(1).cloned().collect();
        let mut columns: Vec<(Locale, usize)> = Vec::with_capacity(model.locales.len());
        for locale in &model.locales {
            let idx = available
                .iter()
                .position(|h| h == locale)
                .map(|i| i + 1)
                .ok_or_else(|| ConversionError::LocaleNotInHeader {
                    locale: locale.clone(),
                    available: available.clone(),
                })?;
            columns.push((locale.clone(), idx));
        }

        let mut seen: IndexMap<String, ()> = IndexMap::new();
        for (offset, row) in body.iter().enumerate() {
            let key = match row.first() {
                Some(k) if !k.is_empty() => k,
                _ => continue,
            };
            if seen.insert(key.clone(), ()).is_some() {
                return Err(ConversionError::DuplicateKey {
                    key: key.clone(),
                    row: offset + 2,
                });
            }
            for (locale, idx) in &columns {
                match row.get(*idx) {
                    Some(cell) if !cell.is_empty() => {
                        model
                            .catalogs
                            .get_mut(locale)
                            .expect("locale registered in new()")
                            .insert(key.clone(), cell.clone());
                    }
                    _ => {} // empty cell or short row → missing translation
                }
            }
        }
        Ok(model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(cells: &[&str]) -> Vec<String> {
        cells.iter().map(|s| s.to_string()).collect()
    }

    fn model_en_zh() -> Model {
        let mut m = Model::new(["en", "zh-TW"]);
        m.set_catalog(
            "en",
            Catalog::from_entries([("user.name", "Name"), ("ok", "OK")]),
        )
        .unwrap();
        m.set_catalog(
            "zh-TW",
            Catalog::from_entries([("user.name", "名字"), ("ok", "好")]),
        )
        .unwrap();
        m
    }

    // --- construction

    #[test]
    fn new_creates_empty_catalog_per_locale_and_dedups() {
        let m = Model::new(["en", "zh-TW", "en"]);
        assert_eq!(m.locales(), ["en", "zh-TW"]);
        assert_eq!(m.source_locale().map(String::as_str), Some("en"));
        assert!(m.catalog("zh-TW").unwrap().is_empty());
    }

    #[test]
    fn set_catalog_rejects_unknown_locale() {
        let mut m = Model::new(["en"]);
        assert_eq!(
            m.set_catalog("fr", Catalog::new()).unwrap_err(),
            ConversionError::UnknownLocale("fr".into())
        );
    }

    // --- to_table (push)

    #[test]
    fn to_table_header_and_source_order() {
        let t = model_en_zh().to_table();
        assert_eq!(t[0], row(&["key", "en", "zh-TW"]));
        assert_eq!(t[1], row(&["user.name", "Name", "名字"]));
        assert_eq!(t[2], row(&["ok", "OK", "好"]));
        assert_eq!(t.len(), 3);
    }

    #[test]
    fn to_table_missing_and_empty_both_blank_cells() {
        let mut m = Model::new(["en", "fr"]);
        m.set_catalog("en", Catalog::from_entries([("a", "A"), ("b", "")]))
            .unwrap();
        m.set_catalog("fr", Catalog::from_entries([("a", "")]))
            .unwrap();
        let t = m.to_table();
        assert_eq!(t[1], row(&["a", "A", ""]));
        assert_eq!(t[2], row(&["b", "", ""]));
    }

    #[test]
    fn to_table_appends_orphan_keys_after_source() {
        let mut m = Model::new(["en", "fr", "de"]);
        m.set_catalog("en", Catalog::from_entries([("a", "A")]))
            .unwrap();
        m.set_catalog(
            "fr",
            Catalog::from_entries([("z", "Z-fr"), ("a", "A-fr"), ("y", "Y-fr")]),
        )
        .unwrap();
        m.set_catalog("de", Catalog::from_entries([("y", "Y-de"), ("x", "X-de")]))
            .unwrap();
        assert_eq!(m.orphan_keys(), ["z", "y", "x"]);
        let t = m.to_table();
        assert_eq!(t[1], row(&["a", "A", "A-fr", ""]));
        assert_eq!(t[2], row(&["z", "", "Z-fr", ""]));
        assert_eq!(t[3], row(&["y", "", "Y-fr", "Y-de"]));
        assert_eq!(t[4], row(&["x", "", "", "X-de"]));
    }

    #[test]
    fn to_table_with_no_keys_is_header_only() {
        assert_eq!(Model::new(["en"]).to_table(), vec![row(&["key", "en"])]);
    }

    // --- from_table (pull)

    #[test]
    fn from_table_basic() {
        let rows = vec![
            row(&["key", "en", "zh-TW"]),
            row(&["user.name", "Name", "名字"]),
            row(&["ok", "OK", "好"]),
        ];
        let m = Model::from_table(&rows, ["en", "zh-TW"]).unwrap();
        assert_eq!(m, model_en_zh());
    }

    #[test]
    fn from_table_matches_columns_by_header_not_position() {
        let rows = vec![
            row(&["key", "zh-TW", "notes", "en"]),
            row(&["ok", "好", "ignored", "OK"]),
        ];
        let m = Model::from_table(&rows, ["en", "zh-TW"]).unwrap();
        assert_eq!(m.locales(), ["en", "zh-TW"]);
        assert_eq!(m.catalog("en").unwrap().get("ok"), Some("OK"));
        assert_eq!(m.catalog("zh-TW").unwrap().get("ok"), Some("好"));
    }

    #[test]
    fn from_table_first_column_is_key_regardless_of_header_text() {
        let rows = vec![row(&["ID", "en"]), row(&["ok", "OK"])];
        let m = Model::from_table(&rows, ["en"]).unwrap();
        assert_eq!(m.catalog("en").unwrap().get("ok"), Some("OK"));
    }

    #[test]
    fn from_table_header_match_is_exact_and_first_wins_and_skips_key_column() {
        // exact: "en " (trailing space) is not "en"
        let rows = vec![row(&["key", "en "])];
        assert!(matches!(
            Model::from_table(&rows, ["en"]).unwrap_err(),
            ConversionError::LocaleNotInHeader { .. }
        ));
        // first match wins on duplicate headers
        let rows = vec![row(&["key", "en", "en"]), row(&["a", "first", "second"])];
        let m = Model::from_table(&rows, ["en"]).unwrap();
        assert_eq!(m.catalog("en").unwrap().get("a"), Some("first"));
        // the key column never matches a locale
        let rows = vec![row(&["key", "en"])];
        assert!(matches!(
            Model::from_table(&rows, ["key"]).unwrap_err(),
            ConversionError::LocaleNotInHeader { .. }
        ));
    }

    #[test]
    fn round_trip_is_order_insensitive_for_non_source_locales() {
        let mut m = Model::new(["en", "zh"]);
        m.set_catalog("en", Catalog::from_entries([("a", "A"), ("b", "B")]))
            .unwrap();
        m.set_catalog("zh", Catalog::from_entries([("b", "乙"), ("a", "甲")]))
            .unwrap();
        let back = Model::from_table(&m.to_table(), ["en", "zh"]).unwrap();
        // Source order is restored exactly; zh comes back in Source (row) order.
        assert_eq!(back.catalog("en").unwrap(), m.catalog("en").unwrap());
        let zh: Vec<_> = back.catalog("zh").unwrap().keys().collect();
        assert_eq!(zh, ["a", "b"]);
        assert_eq!(back.catalog("zh").unwrap().entries().len(), 2);
    }

    #[test]
    fn from_table_missing_locale_lists_available_columns() {
        let rows = vec![row(&["key", "English", "zh-TW"])];
        let err = Model::from_table(&rows, ["en", "zh-TW"]).unwrap_err();
        assert_eq!(
            err,
            ConversionError::LocaleNotInHeader {
                locale: "en".into(),
                available: vec!["English".into(), "zh-TW".into()],
            }
        );
    }

    #[test]
    fn from_table_empty_cell_and_short_row_are_missing() {
        let rows = vec![
            row(&["key", "en", "fr"]),
            row(&["a", "A", ""]),
            row(&["b", "B"]),
            row(&["c"]),
        ];
        let m = Model::from_table(&rows, ["en", "fr"]).unwrap();
        let fr = m.catalog("fr").unwrap();
        assert!(!fr.contains_key("a"));
        assert!(!fr.contains_key("b"));
        assert!(!fr.contains_key("c"));
        let en = m.catalog("en").unwrap();
        assert_eq!(en.get("b"), Some("B"));
        assert!(!en.contains_key("c"));
    }

    #[test]
    fn from_table_skips_rows_with_empty_key() {
        let rows = vec![
            row(&["key", "en"]),
            row(&["a", "A"]),
            row(&["", "spacer"]),
            row(&[]),
            row(&["b", "B"]),
        ];
        let m = Model::from_table(&rows, ["en"]).unwrap();
        let keys: Vec<_> = m.catalog("en").unwrap().keys().collect();
        assert_eq!(keys, ["a", "b"]);
    }

    #[test]
    fn from_table_duplicate_key_reports_row() {
        let rows = vec![
            row(&["key", "en"]),
            row(&["a", "A"]),
            row(&["b", "B"]),
            row(&["a", "A2"]),
        ];
        assert_eq!(
            Model::from_table(&rows, ["en"]).unwrap_err(),
            ConversionError::DuplicateKey {
                key: "a".into(),
                row: 4
            }
        );
    }

    #[test]
    fn from_table_empty_is_error() {
        assert_eq!(
            Model::from_table(&Vec::new(), ["en"]).unwrap_err(),
            ConversionError::EmptySheet
        );
    }

    #[test]
    fn from_table_header_only_gives_empty_catalogs() {
        let m = Model::from_table(&vec![row(&["key", "en"])], ["en"]).unwrap();
        assert!(m.catalog("en").unwrap().is_empty());
    }

    #[test]
    fn from_table_preserves_surrounding_whitespace() {
        let rows = vec![row(&["key", "en"]), row(&["a", "  padded "])];
        let m = Model::from_table(&rows, ["en"]).unwrap();
        assert_eq!(m.catalog("en").unwrap().get("a"), Some("  padded "));
    }

    // --- round trip

    #[test]
    fn round_trip_model_table_model() {
        let mut m = Model::new(["en", "fr"]);
        m.set_catalog("en", Catalog::from_entries([("a", "A"), ("b", "B")]))
            .unwrap();
        m.set_catalog("fr", Catalog::from_entries([("a", "A-fr"), ("z", "Z-fr")]))
            .unwrap();
        let back = Model::from_table(&m.to_table(), ["en", "fr"]).unwrap();
        assert_eq!(back, m);
    }
}
