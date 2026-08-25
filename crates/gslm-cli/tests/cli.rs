use gslm_cli::{ColorChoice, RunOptions, SheetsOverride, run};
use serde_json::json;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use wiremock::matchers::{body_json, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[derive(Clone)]
struct SharedWriter(Arc<Mutex<Vec<u8>>>);

impl Write for SharedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn schema_writes_the_config_schema_to_stdout() {
    let stdout = Arc::new(Mutex::new(Vec::new()));
    let stderr = Arc::new(Mutex::new(Vec::new()));
    let options = RunOptions {
        stdout: Box::new(SharedWriter(stdout.clone())),
        stderr: Box::new(SharedWriter(stderr)),
        ..RunOptions::default()
    };

    assert_eq!(run(vec!["gslm".into(), "schema".into()], options), 0);
    let actual: serde_json::Value = serde_json::from_slice(&stdout.lock().unwrap()).unwrap();
    assert_eq!(actual, gslm_config::schema());
}

#[test]
fn version_uses_the_embedding_package_version() {
    let stdout = Arc::new(Mutex::new(Vec::new()));
    let options = RunOptions {
        stdout: Box::new(SharedWriter(stdout.clone())),
        version: Some("9.8.7"),
        ..RunOptions::default()
    };

    assert_eq!(run(vec!["gslm".into(), "--version".into()], options), 0);
    assert_eq!(
        String::from_utf8(stdout.lock().unwrap().clone()).unwrap(),
        "gslm 9.8.7\n"
    );
}

fn project(format: &str) -> TempDir {
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("gslm.toml"),
        format!(
            r#"version = 1
sheet = "sheet-id"
tab = "i18n"
locales = ["en", "zh-TW"]
path = "locales/{{locale}}.json"
format = "{format}"
"#
        ),
    )
    .unwrap();
    project
}

fn multi_project() -> TempDir {
    let project = tempfile::tempdir().unwrap();
    fs::write(
        project.path().join("gslm.toml"),
        r#"version = 1
path = "locales/{locale}.json"
format = "nest"

[[targets]]
name = "one"
sheet = "one"
tab = "i18n"
locales = ["en", "zh-TW"]

[[targets]]
name = "two"
sheet = "two"
tab = "i18n"
locales = ["en", "zh-TW"]
"#,
    )
    .unwrap();
    project
}

fn execute(project: &Path, server: &MockServer, args: &[&str]) -> (i32, String, String) {
    execute_with_display_options(project, server, args, None, None)
}

fn execute_with_display_options(
    project: &Path,
    server: &MockServer,
    args: &[&str],
    color: Option<ColorChoice>,
    is_tty: Option<bool>,
) -> (i32, String, String) {
    execute_with_sheets(project, server, args, color, is_tty, Some("test-token"))
}

fn execute_without_static_token(
    project: &Path,
    server: &MockServer,
    args: &[&str],
) -> (i32, String, String) {
    execute_with_sheets(project, server, args, None, None, None)
}

fn execute_with_sheets(
    project: &Path,
    server: &MockServer,
    args: &[&str],
    color: Option<ColorChoice>,
    is_tty: Option<bool>,
    access_token: Option<&str>,
) -> (i32, String, String) {
    let stdout = Arc::new(Mutex::new(Vec::new()));
    let stderr = Arc::new(Mutex::new(Vec::new()));
    let options = RunOptions {
        cwd: project.to_path_buf(),
        stdout: Box::new(SharedWriter(stdout.clone())),
        stderr: Box::new(SharedWriter(stderr.clone())),
        sheets: SheetsOverride {
            base_url: Some(server.uri()),
            access_token: access_token.map(str::to_owned),
        },
        color,
        is_tty,
        ..RunOptions::default()
    };
    let argv = std::iter::once("gslm")
        .chain(args.iter().copied())
        .map(String::from)
        .collect();
    let code = std::thread::spawn(move || run(argv, options))
        .join()
        .unwrap();
    let stdout = String::from_utf8(stdout.lock().unwrap().clone()).unwrap();
    let stderr = String::from_utf8(stderr.lock().unwrap().clone()).unwrap();
    (code, stdout, stderr)
}

#[tokio::test]
async fn help_uses_the_stable_program_name_and_explains_commands_and_options() {
    let server = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    let (code, stdout, _) = execute(project.path(), &server, &["--help"]);

    assert_eq!(code, 0);
    assert!(stdout.contains("用法：gslm"));
    assert!(stdout.contains("從 Google Sheets 下載 Catalog"));
    assert!(stdout.contains("設定檔路徑"));
    assert!(!stdout.contains("Usage:"));

    let (code, pull_help, _) = execute(project.path(), &server, &["pull", "--help"]);
    assert_eq!(code, 0);
    assert!(pull_help.contains("用法：gslm pull"));
    assert!(pull_help.contains("覆寫服務帳號憑證檔路徑"));
    assert!(!pull_help.contains("Options:"));
}

#[tokio::test]
async fn explicit_color_flag_overrides_the_embedding_default() {
    let server = MockServer::start().await;
    let project = project("nest");
    fs::create_dir(project.path().join("locales")).unwrap();
    fs::write(
        project.path().join("locales/en.json"),
        "{\"title\":\"Title\"}\n",
    )
    .unwrap();

    let (code, _, stderr) = execute_with_display_options(
        project.path(),
        &server,
        &["push", "--dry-run", "--color", "never"],
        Some(ColorChoice::Always),
        Some(true),
    );

    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.contains("找不到 zh-TW 的 Catalog"));
    assert!(!stderr.contains("\u{1b}["));
}

#[tokio::test]
async fn init_existing_file_keeps_the_io_cause_in_the_error_message() {
    let server = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    fs::write(project.path().join("gslm.toml"), "version = 1\n").unwrap();

    let (code, _, stderr) = execute(project.path(), &server, &["init"]);

    assert_eq!(code, 1);
    assert!(stderr.contains("[IO]"));
    assert!(stderr.contains("設定檔已存在；可加 --force 覆寫"));
}

#[tokio::test]
async fn credential_errors_keep_safe_configuration_details() {
    let server = MockServer::start().await;
    let project = project("nest");
    let (code, _, stderr) = execute_without_static_token(
        project.path(),
        &server,
        &["pull", "--credentials", "missing-service-account.json"],
    );

    assert_eq!(code, 1);
    assert!(stderr.contains("[CREDENTIALS]"), "{stderr}");
    assert!(stderr.contains("Google Sheets 憑證無效：cannot read"));
    assert!(stderr.contains("missing-service-account.json"));
}

fn sheet(rows: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({ "values": rows }))
}

#[tokio::test]
async fn pull_writes_nested_catalogs_creates_directories_and_detects_unchanged() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(sheet(json!([
            ["key", "en", "zh-TW"],
            ["app.title", "Title", "標題"],
            ["missing", "Present", ""]
        ])))
        .expect(3)
        .mount(&server)
        .await;
    let project = project("nest");

    let (code, _, stderr) = execute(project.path(), &server, &["pull"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.contains("新增 2、變更 0、未變動 0"));
    assert_eq!(
        fs::read_to_string(project.path().join("locales/en.json")).unwrap(),
        "{\n  \"app\": {\n    \"title\": \"Title\"\n  },\n  \"missing\": \"Present\"\n}\n"
    );
    assert_eq!(
        fs::read_to_string(project.path().join("locales/zh-TW.json")).unwrap(),
        "{\n  \"app\": {\n    \"title\": \"標題\"\n  }\n}\n"
    );
    let (code, _, stderr) = execute(project.path(), &server, &["pull", "--verbose"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.contains("詳細：讀取 Sheet sheet-id 的 Tab i18n"));
    assert!(stderr.contains("Catalog："));

    let (code, _, stderr) = execute(project.path(), &server, &["pull"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.contains("未變動 2"));
}

#[cfg(unix)]
#[tokio::test]
async fn pull_preserves_existing_catalog_permissions_and_uses_regular_permissions_for_new_files() {
    use std::os::unix::fs::PermissionsExt;

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(sheet(json!([
            ["key", "en", "zh-TW"],
            ["app.title", "Title", "標題"]
        ])))
        .expect(1)
        .mount(&server)
        .await;
    let project = project("nest");
    let locales = project.path().join("locales");
    fs::create_dir(&locales).unwrap();
    let existing = locales.join("en.json");
    fs::write(&existing, "{\"old\":\"value\"}\n").unwrap();
    fs::set_permissions(&existing, fs::Permissions::from_mode(0o644)).unwrap();

    let (code, _, stderr) = execute(project.path(), &server, &["pull"]);

    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.contains("新增 1、變更 1、未變動 0"));
    assert_eq!(
        fs::metadata(existing).unwrap().permissions().mode() & 0o777,
        0o644
    );
    assert_eq!(
        fs::metadata(locales.join("zh-TW.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o644
    );
}

#[tokio::test]
async fn pull_writes_flat_and_dry_run_keeps_the_filesystem_untouched() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(sheet(json!([
            ["key", "en", "zh-TW"],
            ["app.title", "Title", "標題"]
        ])))
        .expect(2)
        .mount(&server)
        .await;
    let project = project("flat");

    let (code, stdout, stderr) = execute(project.path(), &server, &["pull", "--dry-run"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("預覽模式"));
    assert!(!project.path().join("locales/en.json").exists());

    let (code, _, stderr) = execute(project.path(), &server, &["pull"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        fs::read_to_string(project.path().join("locales/en.json")).unwrap(),
        "{\n  \"app.title\": \"Title\"\n}\n"
    );
}

#[tokio::test]
async fn pull_rejects_empty_sheet_before_overwriting_local_catalog_unless_forced() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(sheet(json!([["key", "en", "zh-TW"]])))
        .expect(2)
        .mount(&server)
        .await;
    let project = project("nest");
    fs::create_dir(project.path().join("locales")).unwrap();
    let local = project.path().join("locales/en.json");
    fs::write(&local, "{\"kept\":\"value\"}\n").unwrap();

    let (code, _, stderr) = execute(project.path(), &server, &["pull"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("[PULL_EMPTY_SHEET]"));
    assert!(fs::read_to_string(&local).unwrap().contains("kept"));

    let (code, _, stderr) = execute(project.path(), &server, &["pull", "--force"]);
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(fs::read_to_string(local).unwrap(), "{}\n");
}

#[tokio::test]
async fn pull_allows_an_empty_sheet_when_no_local_catalog_has_keys() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(sheet(json!([["key", "en", "zh-TW"]])))
        .expect(1)
        .mount(&server)
        .await;
    let project = project("nest");

    let (code, _, stderr) = execute(project.path(), &server, &["pull"]);

    assert_eq!(code, 0, "{stderr}");
    assert_eq!(
        fs::read_to_string(project.path().join("locales/en.json")).unwrap(),
        "{}\n"
    );
    assert_eq!(
        fs::read_to_string(project.path().join("locales/zh-TW.json")).unwrap(),
        "{}\n"
    );
}

#[tokio::test]
async fn pull_counts_keys_across_every_existing_catalog_before_an_empty_sheet() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(sheet(json!([["key", "en", "zh-TW"]])))
        .expect(1)
        .mount(&server)
        .await;
    let project = project("nest");
    fs::create_dir(project.path().join("locales")).unwrap();
    fs::write(project.path().join("locales/en.json"), r#"{"first":"one"}"#).unwrap();
    fs::write(
        project.path().join("locales/zh-TW.json"),
        r#"{"second":"two"}"#,
    )
    .unwrap();

    let (code, _, stderr) = execute(project.path(), &server, &["pull"]);

    assert_eq!(code, 1);
    assert!(stderr.contains("[PULL_EMPTY_SHEET]"));
    assert!(stderr.contains("本地有 2 個 key"));
}

#[cfg(unix)]
#[tokio::test]
async fn pull_reports_a_broken_catalog_path_instead_of_treating_it_as_local_data() {
    use std::os::unix::fs::symlink;

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(sheet(json!([["key", "en", "zh-TW"]])))
        .expect(1)
        .mount(&server)
        .await;
    let project = project("nest");
    let locales = project.path().join("locales");
    fs::create_dir(&locales).unwrap();
    let broken = locales.join("en.json");
    symlink(&broken, &broken).unwrap();

    let (code, _, stderr) = execute(project.path(), &server, &["pull"]);

    assert_eq!(code, 1);
    assert!(stderr.contains("[CATALOG]"));
    assert!(stderr.contains(&broken.display().to_string()));
    assert!(!stderr.contains("[PULL_EMPTY_SHEET]"));
}

#[tokio::test]
async fn pull_treats_an_unreadable_existing_catalog_as_data_to_protect() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(sheet(json!([["key", "en", "zh-TW"]])))
        .expect(1)
        .mount(&server)
        .await;
    let project = project("nest");
    let locales = project.path().join("locales");
    fs::create_dir(&locales).unwrap();
    let unreadable_catalog = locales.join("en.json");
    fs::create_dir(&unreadable_catalog).unwrap();

    let (code, _, stderr) = execute(project.path(), &server, &["pull"]);

    assert_eq!(code, 1);
    assert!(stderr.contains("[PULL_EMPTY_SHEET]"));
    assert!(stderr.contains("本地有 1 個 key"));
    assert!(unreadable_catalog.is_dir());
}

#[tokio::test]
async fn push_writes_orphans_at_the_end_and_reports_them() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(body_json(json!({
            "range": "'i18n'!A1",
            "majorDimension": "ROWS",
            "values": [["key", "en", "zh-TW"], ["a", "A", "甲"], ["orphan", "", "孤兒"]]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    let project = project("nest");
    fs::create_dir(project.path().join("locales")).unwrap();
    fs::write(project.path().join("locales/en.json"), "{\"a\":\"A\"}\n").unwrap();
    fs::write(
        project.path().join("locales/zh-TW.json"),
        "{\"a\":\"甲\",\"orphan\":\"孤兒\"}\n",
    )
    .unwrap();

    let (code, _, stderr) = execute(project.path(), &server, &["push"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.contains("孤立 key"));
}

#[tokio::test]
async fn push_explains_when_the_tab_was_cleared_before_a_write_failure() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(503).set_body_json(json!({
            "error": { "code": 503, "message": "backend", "status": "UNAVAILABLE" }
        })))
        .expect(1)
        .mount(&server)
        .await;
    let project = project("nest");
    fs::create_dir(project.path().join("locales")).unwrap();
    fs::write(project.path().join("locales/en.json"), "{\"ok\":\"OK\"}\n").unwrap();
    fs::write(
        project.path().join("locales/zh-TW.json"),
        "{\"ok\":\"好\"}\n",
    )
    .unwrap();

    let (code, _, stderr) = execute(project.path(), &server, &["push"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("[WRITE_AFTER_CLEAR_FAILED]"));
    assert!(stderr.contains("Tab 已被清空但寫入失敗，請重試 push"));
}

#[tokio::test]
async fn push_protects_empty_local_and_strict_rejects_orphans_and_shape_drift() {
    let server = MockServer::start().await;
    let project = project("flat");
    let (code, _, stderr) = execute(project.path(), &server, &["push"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("[PUSH_EMPTY_LOCAL]"));

    fs::create_dir(project.path().join("locales")).unwrap();
    fs::write(
        project.path().join("locales/en.json"),
        "{\"nested\":{\"a\":\"A\"}}\n",
    )
    .unwrap();
    fs::write(
        project.path().join("locales/zh-TW.json"),
        "{\"orphan\":\"孤兒\"}\n",
    )
    .unwrap();
    let (code, stdout, stderr) = execute(project.path(), &server, &["push", "--dry-run"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("預覽模式"));
    assert!(stdout.contains("孤立 key"));
    assert!(stderr.contains("實際形狀"));
    let (code, _, stderr) = execute(project.path(), &server, &["push", "--strict"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("[PUSH_STRICT]"));
    assert!(stderr.contains("實際形狀"));
    assert!(stderr.contains("孤立 key"));
}

#[tokio::test]
async fn push_bad_json_names_the_catalog_path_and_dry_run_skips_writes() {
    let server = MockServer::start().await;
    let project = project("nest");
    fs::create_dir(project.path().join("locales")).unwrap();
    let bad = project.path().join("locales/en.json");
    fs::write(&bad, "not json").unwrap();
    let (code, _, stderr) = execute(project.path(), &server, &["push"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("[CATALOG]"));
    assert!(stderr.contains(&bad.display().to_string()));

    fs::write(&bad, "{\"ok\":\"OK\"}").unwrap();
    fs::write(project.path().join("locales/zh-TW.json"), "{\"ok\":\"好\"}").unwrap();
    let (code, stdout, stderr) = execute(project.path(), &server, &["push", "--dry-run"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.contains("預覽模式"));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn multiple_targets_can_be_filtered_and_ambiguous_overrides_are_rejected() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(sheet(json!([["key", "en", "zh-TW"], ["a", "A", "甲"]])))
        .expect(2)
        .mount(&server)
        .await;
    let project = multi_project();
    let (code, _, stderr) = execute(project.path(), &server, &["pull", "--target", "two"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.contains("目標 two"));
    assert!(!stderr.contains("目標 one"));
    let (code, _, stderr) = execute(
        project.path(),
        &server,
        &["pull", "--target", "two", "--sheet", "override"],
    );
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.contains("目標 two：Sheet=override"));
    let (code, _, stderr) = execute(project.path(), &server, &["pull", "--sheet", "override"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("[CONFIG_INVALID]"));
    let (code, _, stderr) = execute(project.path(), &server, &["pull", "--target", ""]);
    assert_eq!(code, 1);
    assert!(stderr.contains("找不到 Target"));
}

#[tokio::test]
async fn quiet_pull_suppresses_target_summary_and_file_progress() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(sheet(json!([["key", "en", "zh-TW"], ["a", "A", "甲"]])))
        .expect(1)
        .mount(&server)
        .await;
    let project = project("nest");

    let (code, stdout, stderr) = execute(project.path(), &server, &["--quiet", "pull"]);

    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.is_empty());
    assert!(stderr.is_empty(), "{stderr}");
}

#[tokio::test]
async fn no_dotenv_keeps_config_values_for_sync_commands() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(sheet(json!([["key", "en", "zh-TW"], ["a", "A", "甲"]])))
        .expect(1)
        .mount(&server)
        .await;
    let project = project("nest");
    fs::write(project.path().join(".env"), "GSLM_SHEET=from-dotenv\n").unwrap();

    let (code, _, stderr) = execute(project.path(), &server, &["--no-dotenv", "pull"]);

    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.contains("Sheet=sheet-id"));
    assert!(!stderr.contains("Sheet=from-dotenv"));
}

#[tokio::test]
async fn init_template_loads_and_usage_version_and_round_trip_work() {
    let server = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();
    let (code, _, stderr) = execute(project.path(), &server, &["init"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(
        gslm_config::load(gslm_config::LoadOptions {
            cwd: project.path().to_path_buf(),
            env: Default::default(),
            ..Default::default()
        })
        .is_ok()
    );
    let (code, _, _) = execute(project.path(), &server, &["unknown"]);
    assert_eq!(code, 2);
    let (code, stdout, _) = execute(project.path(), &server, &["--version"]);
    assert_eq!(code, 0);
    assert!(stdout.starts_with("gslm "));

    fs::write(
        project.path().join("gslm.toml"),
        r#"version = 1
sheet = "sheet-id"
tab = "i18n"
locales = ["en", "zh-TW"]
path = "locales/{locale}.json"
format = "nest"
"#,
    )
    .unwrap();
    Mock::given(method("GET"))
        .respond_with(sheet(json!([["key", "en", "zh-TW"], ["a", "A", "甲"]])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    let (code, _, stderr) = execute(project.path(), &server, &["pull"]);
    assert_eq!(code, 0, "{stderr}");
    let (code, _, stderr) = execute(project.path(), &server, &["push"]);
    assert_eq!(code, 0, "{stderr}");
}

#[tokio::test]
async fn command_errors_and_init_jsonc_keep_exit_codes_and_output_channels_stable() {
    let server = MockServer::start().await;
    let project = tempfile::tempdir().unwrap();

    let (code, stdout, stderr) = execute(project.path(), &server, &[]);
    assert_eq!(code, 2);
    assert!(stdout.is_empty());
    assert!(stderr.contains("同步 Google Sheets 與本地 i18n Catalog"));
    assert!(stderr.contains("用法：gslm"));

    let (code, stdout, stderr) = execute(project.path(), &server, &["migrate"]);
    assert_eq!(code, 1);
    assert!(stdout.is_empty());
    assert!(stderr.contains("[MIGRATE_JS_ONLY]"));
    assert!(stderr.contains("JS 入口執行 migrate"));

    let (code, stdout, stderr) = execute(
        project.path(),
        &server,
        &["--quiet", "init", "--format", "jsonc"],
    );
    assert_eq!(code, 0, "{stderr}");
    assert!(stdout.is_empty());
    assert!(stderr.is_empty());
    let template = fs::read_to_string(project.path().join("gslm.jsonc")).unwrap();
    assert!(template.contains("\"$schema\""));
    let config = gslm_config::load(gslm_config::LoadOptions {
        cwd: project.path().to_path_buf(),
        env: Default::default(),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(config.targets[0].name, "main");

    let legacy_project = tempfile::tempdir().unwrap();
    fs::write(
        legacy_project.path().join("gslm.config.cjs"),
        "module.exports = {}\n",
    )
    .unwrap();
    let (code, _, stderr) = execute(legacy_project.path(), &server, &["init"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("[MIGRATE_JS_ONLY]"));
}

#[tokio::test]
async fn push_strict_mode_enforces_each_protection_independently() {
    let server = MockServer::start().await;
    let empty = project("nest");
    let (code, _, stderr) = execute(empty.path(), &server, &["push", "--dry-run"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("[PUSH_EMPTY_LOCAL]"));
    assert_eq!(
        execute(empty.path(), &server, &["push", "--dry-run", "--force"]).0,
        0
    );

    let orphan = project("nest");
    fs::create_dir(orphan.path().join("locales")).unwrap();
    fs::write(orphan.path().join("locales/en.json"), r#"{"key":"source"}"#).unwrap();
    fs::write(
        orphan.path().join("locales/zh-TW.json"),
        r#"{"orphan":"孤立"}"#,
    )
    .unwrap();
    let (code, _, stderr) = execute(orphan.path(), &server, &["push", "--dry-run"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(stderr.contains("孤立 key：orphan"));
    let (code, _, stderr) = execute(orphan.path(), &server, &["push", "--dry-run", "--strict"]);
    assert_eq!(code, 1);
    assert!(stderr.contains("[PUSH_STRICT]"));

    let clean = project("nest");
    fs::create_dir(clean.path().join("locales")).unwrap();
    fs::write(clean.path().join("locales/en.json"), r#"{"key":"source"}"#).unwrap();
    fs::write(clean.path().join("locales/zh-TW.json"), r#"{"key":"翻譯"}"#).unwrap();
    let (code, _, stderr) = execute(clean.path(), &server, &["push", "--dry-run", "--strict"]);
    assert_eq!(code, 0, "{stderr}");
    assert!(!stderr.contains("孤立 key"));
}
