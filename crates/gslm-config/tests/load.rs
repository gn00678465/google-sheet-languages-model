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
        "locales": ["en"],
        "path": "translations.json"
    })));
}
