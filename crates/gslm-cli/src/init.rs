use crate::CliError;
use crate::args::{InitArgs, InitFormat};
use std::fs;
use std::path::Path;

const TOML_TEMPLATE: &str = r#"#:schema https://raw.githubusercontent.com/gn00678465/google-sheet-languages-model/main/docs/schema/v1.json
# gslm 設定檔。第一個 Locale 是來源語言，決定 Sheet 的 key 順序。
version = 1

[[targets]]
name = "main"
sheet = "請填入 Google Sheet ID"
tab = "Sheet1"
locales = ["en", "zh-TW"]
path = "locales/{locale}.json"
format = "nest"
key_separator = "."

# 可改成 credentials.env = "GSLM_CREDENTIALS_JSON"。
[targets.credentials]
file = "./service-account.json"
"#;

const JSONC_TEMPLATE: &str = r#"{
  "$schema": "https://raw.githubusercontent.com/gn00678465/google-sheet-languages-model/main/docs/schema/v1.json",
  "version": 1,
  "targets": [
    {
      "name": "main",
      "sheet": "請填入 Google Sheet ID",
      "tab": "Sheet1",
      "locales": ["en", "zh-TW"],
      "path": "locales/{locale}.json",
      "format": "nest",
      "key_separator": ".",
      "credentials": { "file": "./service-account.json" }
    }
  ]
}
"#;

pub(crate) fn run(cwd: &Path, args: &InitArgs) -> Result<std::path::PathBuf, CliError> {
    let (filename, template) = match args.format {
        InitFormat::Toml => ("gslm.toml", TOML_TEMPLATE),
        InitFormat::Jsonc => ("gslm.jsonc", JSONC_TEMPLATE),
    };
    let path = cwd.join(filename);
    if path.exists() && !args.force {
        return Err(CliError::Io {
            path,
            source: std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                "設定檔已存在；可加 --force 覆寫",
            ),
        });
    }
    if !args.force {
        for legacy in [
            "gslm.config.js",
            "gslm.config.ts",
            "gslm.config.mjs",
            "gslm.config.cjs",
        ] {
            if cwd.join(legacy).is_file() {
                return Err(CliError::MigrateViaJs);
            }
        }
    }
    fs::write(&path, template)
        .map(|_| path.clone())
        .map_err(|source| CliError::Io { path, source })
}
