use gslm_config::{ConfigError, CredentialsSource, LoadOptions, Overrides, load, schema};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use tempfile::tempdir;

fn options(cwd: &std::path::Path) -> LoadOptions {
    LoadOptions {
        cwd: cwd.to_path_buf(),
        env: BTreeMap::new(),
        ..LoadOptions::default()
    }
}

#[test]
fn loads_toml_and_resolves_paths_relative_to_config() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("gslm.toml"),
        r#"version = 1
sheet = "sheet-id"
tab = "Common"
locales = ["en", "zh-TW"]
path = "locales/{locale}/common.json"
"#,
    )
    .unwrap();
    let subdir = project.path().join("packages/web");
    fs::create_dir_all(&subdir).unwrap();

    let config = load(options(&subdir)).unwrap();

    assert_eq!(config.targets.len(), 1);
    let target = &config.targets[0];
    assert_eq!(target.name, "default");
    assert_eq!(target.sheet, "sheet-id");
    assert_eq!(target.tab, "Common");
    assert_eq!(target.locales, ["en", "zh-TW"]);
    assert_eq!(
        target.path,
        project.path().join("locales/{locale}/common.json")
    );
    assert_eq!(target.credentials, CredentialsSource::ApplicationDefault);
}

#[test]
fn discovers_jsonc_in_a_monorepo_and_loads_dotenv_without_overriding_injected_env() {
    let project = tempdir().unwrap();
    fs::create_dir(project.path().join(".git")).unwrap();
    fs::write(
        project.path().join("gslm.jsonc"),
        r#"{
          // JSON and JSONC deliberately share the relaxed parser.
          "version": 1,
          "sheet": "from-file",
          "tab": "Common",
          "locales": ["en", "ja",],
          "path": "translations/{locale}.json",
          "credentials": { "env": "SERVICE_ACCOUNT" },
        }"#,
    )
    .unwrap();
    fs::write(
        project.path().join(".env"),
        "SERVICE_ACCOUNT=from-dotenv\nGSLM_SHEET=from-dotenv\n",
    )
    .unwrap();
    let child = project.path().join("apps/web");
    fs::create_dir_all(&child).unwrap();
    let mut opts = options(&child);
    opts.env
        .insert("SERVICE_ACCOUNT".into(), "from-injected-env".into());
    opts.env
        .insert("GSLM_SHEET".into(), "from-injected-env".into());

    let config = load(opts).unwrap();

    assert_eq!(config.config_path, Some(project.path().join("gslm.jsonc")));
    assert_eq!(config.targets[0].sheet, "from-injected-env");
    assert_eq!(
        config.targets[0].credentials,
        CredentialsSource::Json {
            env_name: "SERVICE_ACCOUNT".into(),
            value: "from-injected-env".into(),
        }
    );
}

#[test]
fn accepts_comments_and_trailing_commas_in_json_too() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("gslm.json"),
        r#"{
          // `.json` intentionally shares JSONC's relaxed parsing.
          "version": 1,
          "sheet": "json-sheet",
          "tab": "Main",
          "locales": ["en",],
          "path": "translations/{locale}.json",
        }"#,
    )
    .unwrap();

    let config = load(options(project.path())).unwrap();

    assert_eq!(config.config_path, Some(project.path().join("gslm.json")));
    assert_eq!(config.targets[0].sheet, "json-sheet");
}

#[test]
fn chooses_toml_before_other_formats_and_stops_at_git_boundary() {
    let outer = tempdir().unwrap();
    fs::write(
        outer.path().join("gslm.toml"),
        "version = 1\nsheet = \"outer\"\ntab = \"Outer\"\nlocales = [\"en\"]\npath = \"{locale}.json\"\n",
    )
    .unwrap();
    let project = outer.path().join("project");
    fs::create_dir_all(project.join(".git")).unwrap();
    fs::write(
        project.join("gslm.json"),
        r#"{"version":1,"sheet":"json","tab":"JSON","locales":["en"],"path":"{locale}.json"}"#,
    )
    .unwrap();
    fs::write(
        project.join("gslm.toml"),
        "version = 1\nsheet = \"toml\"\ntab = \"TOML\"\nlocales = [\"en\"]\npath = \"{locale}.json\"\n",
    )
    .unwrap();
    let child = project.join("src/lib");
    fs::create_dir_all(&child).unwrap();

    let config = load(options(&child)).unwrap();

    assert_eq!(config.targets[0].sheet, "toml");
    assert_eq!(config.warnings.len(), 1);
    fs::remove_file(project.join("gslm.toml")).unwrap();
    fs::remove_file(project.join("gslm.json")).unwrap();
    let err = load(options(&child)).unwrap_err();
    assert!(matches!(err, ConfigError::NotFound { .. }));
}

#[test]
fn rejects_legacy_and_invalid_config_with_stable_codes() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("gslm.config.mjs"),
        "export default {}\n",
    )
    .unwrap();
    let err = load(options(project.path())).unwrap_err();
    assert_eq!(err.code(), "CONFIG_LEGACY");
    assert!(err.to_string().contains("gslm migrate"));

    fs::write(
        project.path().join("gslm.toml"),
        "version = 2\nsheet = \"id\"\ntab = \"Main\"\nlocales = [\"en\"]\npath = \"{locale}.json\"\n",
    )
    .unwrap();
    let err = load(options(project.path())).unwrap_err();
    assert_eq!(err.code(), "CONFIG_UNSUPPORTED_VERSION");
    assert!(err.to_string().contains("升級 gslm"));

    fs::write(
        project.path().join("gslm.toml"),
        "version = 1\nsheetId = \"id\"\ntab = \"Main\"\nlocales = [\"en\"]\npath = \"{locale}.json\"\n",
    )
    .unwrap();
    let err = load(options(project.path())).unwrap_err();
    assert_eq!(err.code(), "CONFIG_INVALID");
    assert!(err.to_string().contains("已改為 `sheet`"));

    fs::write(
        project.path().join("gslm.toml"),
        "version = 1\nsheet = \"id\"\ntab = \"Main\"\nlocales = [\"en\"]\npath = \"{locale}.json\"\n[credentials]\ntype = \"service_account\"\n",
    )
    .unwrap();
    let err = load(options(project.path())).unwrap_err();
    assert_eq!(err.code(), "CONFIG_INVALID");
    assert!(err.to_string().contains("credentials.type"));
}

#[test]
fn keeps_dotenv_parse_errors_secret_free() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("gslm.toml"),
        "version = 1\nsheet = \"id\"\ntab = \"Main\"\nlocales = [\"en\"]\npath = \"{locale}.json\"\n",
    )
    .unwrap();
    let secret = "never-show-this-dotenv-secret";
    fs::write(
        project.path().join(".env"),
        format!("SERVICE_ACCOUNT=\"{secret}\n"),
    )
    .unwrap();

    let error = load(options(project.path())).unwrap_err();

    assert_eq!(error.code(), "CONFIG_PARSE");
    assert!(!error.to_string().contains(secret));
}

#[test]
fn expands_targets_and_requires_one_target_for_field_overrides() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("gslm.toml"),
        r#"version = 1
sheet = "shared"
locales = ["en", "zh-TW"]
format = "flat"

[[targets]]
name = "web"
tab = "Web"
path = "web/{locale}.json"

[[targets]]
name = "mobile"
tab = "Mobile"
path = "mobile/{locale}.json"
key_separator = "/"
"#,
    )
    .unwrap();
    let mut opts = options(project.path());
    opts.overrides = Overrides {
        sheet: Some("override".into()),
        ..Overrides::default()
    };
    let err = load(opts).unwrap_err();
    assert!(matches!(err, ConfigError::AmbiguousOverride { .. }));

    let mut opts = options(project.path());
    opts.targets = Some(vec!["mobile".into()]);
    opts.overrides = Overrides {
        sheet: Some("override".into()),
        ..Overrides::default()
    };
    let config = load(opts).unwrap();
    assert_eq!(config.targets.len(), 1);
    assert_eq!(config.targets[0].name, "mobile");
    assert_eq!(config.targets[0].sheet, "override");
    assert_eq!(config.targets[0].format, gslm_core::Format::Flat);
    assert_eq!(config.targets[0].key_separator, "/");

    let mut opts = options(project.path());
    opts.overrides = Overrides {
        credentials: Some("credentials/service-account.json".into()),
        ..Overrides::default()
    };
    let config = load(opts).unwrap();
    assert_eq!(config.targets.len(), 2);
    assert!(config.targets.iter().all(|target| {
        target.credentials
            == CredentialsSource::File(project.path().join("credentials/service-account.json"))
    }));
}

#[test]
fn accepts_complete_cli_mode_but_validates_target_values() {
    let project = tempdir().unwrap();
    let mut opts = options(project.path());
    opts.overrides = Overrides {
        sheet: Some("id".into()),
        tab: Some("Main".into()),
        locales: Some(vec!["en".into(), "zh".into()]),
        path: Some("locale/{locale}.json".into()),
        format: Some(gslm_core::Format::Flat),
        credentials: Some("credentials/service-account.json".into()),
        ..Overrides::default()
    };
    let config = load(opts).unwrap();
    assert_eq!(config.config_path, None);
    assert_eq!(config.targets[0].name, "cli");
    assert_eq!(config.targets[0].format, gslm_core::Format::Flat);
    assert_eq!(
        config.targets[0].credentials,
        CredentialsSource::File(project.path().join("credentials/service-account.json"))
    );

    let mut opts = options(project.path());
    opts.overrides = Overrides {
        sheet: Some("id".into()),
        tab: Some("Main".into()),
        locales: Some(vec!["en".into(), "en".into()]),
        path: Some("translations.json".into()),
        ..Overrides::default()
    };
    let err = load(opts).unwrap_err();
    assert_eq!(err.code(), "CONFIG_INVALID");
    assert!(err.to_string().contains("locales"));
}

#[test]
fn complete_cli_mode_precedes_legacy_discovery() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("gslm.config.mjs"),
        "export default {}\n",
    )
    .unwrap();
    let mut opts = options(project.path());
    opts.overrides = Overrides {
        sheet: Some("id".into()),
        tab: Some("Main".into()),
        locales: Some(vec!["en".into()]),
        path: Some("locales/{locale}.json".into()),
        ..Overrides::default()
    };

    let config = load(opts).unwrap();

    assert_eq!(config.config_path, None);
    assert_eq!(config.targets[0].name, "cli");
}

#[test]
fn rejects_empty_locale_and_treats_empty_environment_values_as_unset() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("gslm.toml"),
        "version = 1\nsheet = \"from-file\"\ntab = \"Main\"\nlocales = [\"en\"]\npath = \"{locale}.json\"\n",
    )
    .unwrap();
    let mut opts = options(project.path());
    opts.env = BTreeMap::from([
        ("GSLM_SHEET".into(), String::new()),
        ("GSLM_LOCALES".into(), String::new()),
        ("GSLM_CREDENTIALS".into(), String::new()),
        ("GSLM_CREDENTIALS_JSON".into(), String::new()),
    ]);
    let config = load(opts).unwrap();
    assert_eq!(config.targets[0].sheet, "from-file");
    assert_eq!(config.targets[0].locales, ["en"]);
    assert_eq!(
        config.targets[0].credentials,
        CredentialsSource::ApplicationDefault
    );

    fs::write(
        project.path().join("gslm.toml"),
        "version = 1\nsheet = \"id\"\ntab = \"Main\"\nlocales = [\"en\"]\npath = \"{locale}.json\"\n[credentials]\nenv = \"SERVICE_ACCOUNT\"\n",
    )
    .unwrap();
    fs::write(project.path().join(".env"), "SERVICE_ACCOUNT=from-dotenv\n").unwrap();
    let mut opts = options(project.path());
    opts.env.insert("SERVICE_ACCOUNT".into(), String::new());
    let config = load(opts).unwrap();
    assert_eq!(
        config.targets[0].credentials,
        CredentialsSource::Json {
            env_name: "SERVICE_ACCOUNT".into(),
            value: "from-dotenv".into(),
        }
    );

    fs::write(
        project.path().join("gslm.toml"),
        "version = 1\nsheet = \"id\"\ntab = \"Main\"\nlocales = [\"\"]\npath = \"{locale}.json\"\n",
    )
    .unwrap();
    let error = load(options(project.path())).unwrap_err();
    assert_eq!(error.code(), "CONFIG_INVALID");
    assert!(error.to_string().contains("不可有空白 Locale"));
}

#[test]
fn deduplicates_requested_targets_and_rejects_selection_without_config() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("gslm.toml"),
        r#"version = 1
sheet = "shared"
locales = ["en"]

[[targets]]
name = "web"
tab = "Web"
path = "web/{locale}.json"

[[targets]]
name = "mobile"
tab = "Mobile"
path = "mobile/{locale}.json"
"#,
    )
    .unwrap();
    let mut opts = options(project.path());
    opts.targets = Some(vec!["mobile".into(), "web".into(), "mobile".into()]);
    let config = load(opts).unwrap();
    assert_eq!(
        config
            .targets
            .iter()
            .map(|target| target.name.as_str())
            .collect::<Vec<_>>(),
        ["mobile", "web"]
    );

    let no_config = tempdir().unwrap();
    let mut opts = options(no_config.path());
    opts.targets = Some(vec!["web".into()]);
    let error = load(opts).unwrap_err();
    assert!(matches!(error, ConfigError::NotFound { .. }));
}

#[test]
fn generated_schema_matches_the_checked_in_v1_contract() {
    let generated = format!("{}\n", serde_json::to_string_pretty(&schema()).unwrap());
    let checked_in = include_str!("../../../docs/schema/v1.json");
    assert_eq!(generated, checked_in, "重新產生 docs/schema/v1.json");

    let validator = jsonschema::validator_for(&schema()).unwrap();
    assert!(validator.is_valid(&json!({
        "version": 1,
        "sheet": "sheet-id",
        "tab": "Main",
        "locales": ["en", "zh-TW"],
        "path": "locales/{locale}.json",
        "credentials": { "file": "./sa.json" }
    })));
    assert!(!validator.is_valid(&json!({
        "version": 1,
        "sheet": "sheet-id",
        "tab": "Main",
        "locales": ["en"],
        "path": "{locale}.json",
        "credentials": { "private_key": "do-not-commit" }
    })));
    assert!(!validator.is_valid(&json!({
        "version": 2,
        "sheet": "sheet-id",
        "tab": "Main",
        "locales": ["en"],
        "path": "{locale}.json"
    })));
    assert!(!validator.is_valid(&json!({
        "version": 1,
        "sheet": "sheet-id",
        "tab": "Main",
        "locales": [],
        "path": "{locale}.json"
    })));
    assert!(!validator.is_valid(&json!({
        "version": 1,
        "sheet": "sheet-id",
        "tab": "Main",
        "locales": [""],
        "path": "{locale}.json"
    })));
    assert!(!validator.is_valid(&json!({
        "version": 1,
        "sheet": "sheet-id",
        "tab": "Main",
        "locales": ["en"],
        "path": "translations.json"
    })));
}

fn complete_config(extra: &str) -> String {
    format!(
        "version = 1\nsheet = \"file-sheet\"\ntab = \"File\"\nlocales = [\"en\", \"zh-TW\"]\npath = \"translations/{{locale}}.json\"\n{extra}"
    )
}

#[test]
fn explicit_paths_and_parse_failures_keep_their_actionable_details() {
    let project = tempdir().unwrap();
    let unsupported = project.path().join("gslm.yaml");
    fs::write(&unsupported, "version: 1\n").unwrap();
    let mut opts = options(project.path());
    opts.config_path = Some(unsupported.clone());
    let error = load(opts).unwrap_err();
    assert_eq!(error.code(), "CONFIG_UNSUPPORTED");
    assert!(
        error
            .to_string()
            .contains(&unsupported.display().to_string())
    );

    let missing = project.path().join("missing.toml");
    let mut opts = options(project.path());
    opts.config_path = Some(missing.clone());
    let error = load(opts).unwrap_err();
    assert!(matches!(
        error,
        ConfigError::NotFound { ref start, ref searched } if start == project.path() && searched == &vec![missing]
    ));

    let bad_toml = project.path().join("bad.toml");
    fs::write(&bad_toml, "version = 1\nsheet =\n").unwrap();
    let mut opts = options(project.path());
    opts.config_path = Some(bad_toml.clone());
    let error = load(opts).unwrap_err();
    assert!(matches!(
        error,
        ConfigError::Parse { ref path, line: Some(_), column: Some(_), .. } if path == &bad_toml
    ));
    assert!(error.to_string().contains("第 2 行"));

    let bad_json = project.path().join("bad.jsonc");
    fs::write(&bad_json, "{ version: 1,").unwrap();
    let mut opts = options(project.path());
    opts.config_path = Some(bad_json.clone());
    let error = load(opts).unwrap_err();
    assert_eq!(error.code(), "CONFIG_PARSE");
    assert!(error.to_string().contains(&bad_json.display().to_string()));

    let wrong_type = project.path().join("wrong-type.toml");
    fs::write(&wrong_type, "version = 1\nsheet = 42\n").unwrap();
    let mut opts = options(project.path());
    opts.config_path = Some(wrong_type.clone());
    let error = load(opts).unwrap_err();
    assert!(matches!(
        error,
        ConfigError::Invalid { ref path, ref field, .. }
            if path.as_ref() == Some(&wrong_type) && field == "config"
    ));
}

#[test]
fn rejects_unsafe_credentials_and_unknown_fields_with_specific_guidance() {
    let project = tempdir().unwrap();
    let cases = [
        (
            complete_config("shte = \"typo\"\n"),
            "未知欄位 shte；是否要用 `sheet`？",
        ),
        (
            complete_config("sheetTitle = \"legacy\"\n"),
            "舊欄位 `sheetTitle` 已改為 `tab`；請執行 `gslm migrate`",
        ),
        (
            complete_config("credentials = \"inline\"\n"),
            "credentials：必須是 { file = \"...\" } 或 { env = \"...\" }",
        ),
        (
            complete_config("[credentials]\nprivate_key = \"secret\"\n"),
            "credentials.private_key",
        ),
        (
            complete_config("[credentials]\nfile = \"one.json\"\nenv = \"SERVICE_ACCOUNT\"\n"),
            "file 與 env 只能擇一",
        ),
    ];

    for (index, (text, expected)) in cases.into_iter().enumerate() {
        let path = project.path().join(format!("case-{index}.toml"));
        fs::write(&path, text).unwrap();
        let mut opts = options(project.path());
        opts.config_path = Some(path);
        let error = load(opts).unwrap_err();
        assert_eq!(error.code(), "CONFIG_INVALID", "case {index}: {error}");
        assert!(
            error.to_string().contains(expected),
            "case {index}: {error} does not contain {expected}"
        );
    }
}

#[test]
fn validates_target_names_required_values_and_path_templates() {
    let project = tempdir().unwrap();
    let cases = vec![
        (
            "version = 1\ntargets = []\n".to_string(),
            "targets：不可為空陣列",
        ),
        (
            "version = 1\n[[targets]]\nsheet = \"id\"\ntab = \"Main\"\nlocales = [\"en\"]\npath = \"{locale}.json\"\n".to_string(),
            "targets.name：每個 Target 都必須有唯一 name",
        ),
        (
            "version = 1\n[[targets]]\nname = \"web\"\nsheet = \"id\"\ntab = \"Main\"\nlocales = [\"en\"]\npath = \"{locale}.json\"\n[[targets]]\nname = \"web\"\nsheet = \"other\"\ntab = \"Other\"\nlocales = [\"en\"]\npath = \"{locale}.json\"\n".to_string(),
            "Target name `web` 重複",
        ),
        (
            complete_config("").replace("locales = [\"en\", \"zh-TW\"]", "locales = []"),
            "targets.default.locales：不可為空陣列",
        ),
        (
            complete_config("").replace("translations/{locale}.json", "translations.json"),
            "必須包含 {locale} 佔位符",
        ),
        (
            complete_config("").replace("translations/{locale}.json", "translations/{language}.json"),
            "不支援 {language}；目前只支援 {locale}",
        ),
        (
            complete_config("key_separator = \"\"\n"),
            "targets.default.key_separator：不可為空字串",
        ),
    ];

    for (index, (text, expected)) in cases.into_iter().enumerate() {
        let path = project.path().join(format!("target-{index}.toml"));
        fs::write(&path, text).unwrap();
        let mut opts = options(project.path());
        opts.config_path = Some(path);
        let error = load(opts).unwrap_err();
        assert_eq!(error.code(), "CONFIG_INVALID", "case {index}: {error}");
        assert!(
            error.to_string().contains(expected),
            "case {index}: {error}"
        );
    }
}

#[test]
fn applies_all_environment_and_cli_overrides_in_documented_precedence() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("gslm.toml"),
        complete_config("format = \"nest\"\n"),
    )
    .unwrap();
    let mut opts = options(project.path());
    opts.env = BTreeMap::from([
        ("GSLM_SHEET".into(), "env-sheet".into()),
        ("GSLM_TAB".into(), "Env".into()),
        ("GSLM_LOCALES".into(), "en, ja ".into()),
        ("GSLM_PATH".into(), "env/{locale}.json".into()),
        ("GSLM_FORMAT".into(), "flat".into()),
        ("GSLM_KEY_SEPARATOR".into(), "/".into()),
        ("GSLM_CREDENTIALS".into(), "env-service-account.json".into()),
    ]);
    opts.overrides = Overrides {
        sheet: Some("cli-sheet".into()),
        tab: Some("Cli".into()),
        locales: Some(vec!["en".into(), "fr".into()]),
        path: Some("cli/{locale}.json".into()),
        format: Some(gslm_core::Format::Nest),
        key_separator: Some(".".into()),
        credentials_json: Some("{\"type\":\"service_account\"}".into()),
        ..Overrides::default()
    };

    let config = load(opts).unwrap();
    let target = &config.targets[0];
    assert_eq!(target.sheet, "cli-sheet");
    assert_eq!(target.tab, "Cli");
    assert_eq!(target.locales, ["en", "fr"]);
    assert_eq!(target.path, project.path().join("cli/{locale}.json"));
    assert_eq!(target.format, gslm_core::Format::Nest);
    assert_eq!(target.key_separator, ".");
    assert_eq!(
        target.credentials,
        CredentialsSource::Json {
            env_name: "GSLM_CREDENTIALS_JSON".into(),
            value: "{\"type\":\"service_account\"}".into(),
        }
    );
}

#[test]
fn rejects_conflicting_environment_credentials_and_names_missing_target() {
    let project = tempdir().unwrap();
    fs::write(project.path().join("gslm.toml"), complete_config("")).unwrap();
    let mut opts = options(project.path());
    opts.env = BTreeMap::from([
        ("GSLM_CREDENTIALS".into(), "file.json".into()),
        ("GSLM_CREDENTIALS_JSON".into(), "secret".into()),
    ]);
    let error = load(opts).unwrap_err();
    assert_eq!(error.code(), "CONFIG_INVALID");
    assert!(
        error
            .to_string()
            .contains("GSLM_CREDENTIALS 與 GSLM_CREDENTIALS_JSON 只能擇一")
    );

    let mut opts = options(project.path());
    opts.targets = Some(vec!["missing".into()]);
    let error = load(opts).unwrap_err();
    assert!(matches!(
        error,
        ConfigError::UnknownTarget { ref name, ref available }
            if name == "missing" && available == &vec!["default".to_string()]
    ));
    assert_eq!(error.code(), "CONFIG_INVALID");
}

#[test]
fn resolves_file_credentials_and_reports_missing_environment_values() {
    let project = tempdir().unwrap();
    fs::write(
        project.path().join("gslm.toml"),
        complete_config("[credentials]\nfile = \"credentials/../service-account.json\"\n"),
    )
    .unwrap();
    let config = load(options(project.path())).unwrap();
    assert_eq!(
        config.targets[0].credentials,
        CredentialsSource::File(project.path().join("service-account.json"))
    );

    fs::write(
        project.path().join("gslm.toml"),
        complete_config("[credentials]\nenv = \"MISSING_SERVICE_ACCOUNT\"\n"),
    )
    .unwrap();
    let error = load(options(project.path())).unwrap_err();
    assert!(
        matches!(error, ConfigError::MissingEnv { ref name } if name == "MISSING_SERVICE_ACCOUNT")
    );
    assert_eq!(error.code(), "CONFIG_INVALID");
    assert!(error.to_string().contains("MISSING_SERVICE_ACCOUNT"));
}

#[test]
fn rejects_conflicting_cli_credentials_and_invalid_format_or_template_syntax() {
    let project = tempdir().unwrap();
    fs::write(project.path().join("gslm.toml"), complete_config("")).unwrap();
    let mut opts = options(project.path());
    opts.overrides = Overrides {
        credentials: Some("service-account.json".into()),
        credentials_json: Some("secret".into()),
        ..Overrides::default()
    };
    let error = load(opts).unwrap_err();
    assert_eq!(error.code(), "CONFIG_INVALID");
    assert!(
        error
            .to_string()
            .contains("credentials 或 credentials_json 其中之一")
    );

    let mut opts = options(project.path());
    opts.env.insert("GSLM_FORMAT".into(), "yaml".into());
    let error = load(opts).unwrap_err();
    assert_eq!(error.code(), "CONFIG_INVALID");
    assert!(
        error
            .to_string()
            .contains("GSLM_FORMAT：只能是 `nest` 或 `flat`")
    );

    for (name, text, expected) in [
        (
            "unclosed-placeholder",
            complete_config("").replace("translations/{locale}.json", "translations/{locale.json"),
            "佔位符必須以 } 結束",
        ),
        (
            "stray-closing-placeholder",
            complete_config("").replace("translations/{locale}.json", "translations/locale}.json"),
            "佔位符格式無效",
        ),
    ] {
        let path = project.path().join(format!("{name}.toml"));
        fs::write(&path, text).unwrap();
        let mut opts = options(project.path());
        opts.config_path = Some(path);
        let error = load(opts).unwrap_err();
        assert_eq!(error.code(), "CONFIG_INVALID", "{name}: {error}");
        assert!(error.to_string().contains(expected), "{name}: {error}");
    }

    let invalid_format = project.path().join("invalid-format.toml");
    fs::write(&invalid_format, complete_config("format = \"yaml\"\n")).unwrap();
    let mut opts = options(project.path());
    opts.config_path = Some(invalid_format.clone());
    let error = load(opts).unwrap_err();
    assert_eq!(error.code(), "CONFIG_INVALID");
    assert!(
        error
            .to_string()
            .contains(&invalid_format.display().to_string())
    );
    assert!(error.to_string().contains("unknown variant `yaml`"));
}

#[test]
fn validates_non_object_targets_and_preserves_error_display_variants() {
    let project = tempdir().unwrap();
    let non_object_target = project.path().join("targets.json");
    fs::write(&non_object_target, r#"{"version":1,"targets":[false]}"#).unwrap();
    let mut opts = options(project.path());
    opts.config_path = Some(non_object_target);
    let error = load(opts).unwrap_err();
    assert_eq!(error.code(), "CONFIG_INVALID");
    assert!(error.to_string().contains("targets[0]：必須是物件"));

    let no_suggestion = project.path().join("unknown.toml");
    fs::write(
        &no_suggestion,
        complete_config("entirely_unrelated = true\n"),
    )
    .unwrap();
    let mut opts = options(project.path());
    opts.config_path = Some(no_suggestion);
    let error = load(opts).unwrap_err();
    assert_eq!(error.code(), "CONFIG_INVALID");
    assert_eq!(error.to_string(), "未知欄位 entirely_unrelated");

    let invalid_value = project.path().join("invalid-value.toml");
    fs::write(&invalid_value, "version = 1\nsheet = 42\n").unwrap();
    let mut opts = options(project.path());
    opts.config_path = Some(invalid_value.clone());
    let error = load(opts).unwrap_err();
    assert_eq!(error.code(), "CONFIG_INVALID");
    assert!(
        error
            .to_string()
            .contains(&invalid_value.display().to_string())
    );
    assert!(error.to_string().contains("config"));
}
