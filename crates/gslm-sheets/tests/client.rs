//! Integration tests against a local mock of the Sheets REST API.
//! No network, no Google, no real credentials.

use gslm_sheets::{Credentials, SheetsClient, SheetsError, TokenProvider};
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const SHEET: &str = "1AbC-def_GHI";

async fn client(server: &MockServer) -> SheetsClient {
    SheetsClient::builder(Credentials::AccessToken("tok-1".into()))
        .base_url(server.uri())
        .build()
        .await
        .unwrap()
}

fn rows(r: &[&[&str]]) -> Vec<Vec<String>> {
    r.iter()
        .map(|row| row.iter().map(|c| c.to_string()).collect())
        .collect()
}

fn google_error(status: u16, message: &str) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_json(json!({
        "error": { "code": status, "message": message, "status": "X" }
    }))
}

// ---------- read

#[tokio::test]
async fn read_tab_returns_rows_and_sends_expected_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(format!("/v4/spreadsheets/{SHEET}/values/%27i18n%27")))
        .and(query_param("majorDimension", "ROWS"))
        .and(query_param("valueRenderOption", "FORMATTED_VALUE"))
        .and(header("authorization", "Bearer tok-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "range": "'i18n'!A1:C3",
            "majorDimension": "ROWS",
            "values": [["key","en","zh"],["ok","OK","好"],["short","S"]]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let table = client(&server).await.read_tab(SHEET, "i18n").await.unwrap();
    assert_eq!(
        table,
        rows(&[&["key", "en", "zh"], &["ok", "OK", "好"], &["short", "S"]])
    );
}

#[tokio::test]
async fn read_tab_empty_values_gives_empty_table() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"range": "'i18n'!A1:Z1000"})))
        .mount(&server)
        .await;
    let table = client(&server).await.read_tab(SHEET, "i18n").await.unwrap();
    assert!(table.is_empty());
}

#[tokio::test]
async fn read_tab_encodes_tab_names_with_spaces_quotes_and_unicode() {
    let server = MockServer::start().await;
    // wiremock matches on the raw (encoded) path, so this also pins the encoding.
    Mock::given(method("GET"))
        .and(path(format!(
            "/v4/spreadsheets/{SHEET}/values/%27it%27%27s%20%E7%BF%BB%E8%AD%AF%20(v2)%27"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"values": [["key"]]})))
        .expect(1)
        .mount(&server)
        .await;
    let table = client(&server)
        .await
        .read_tab(SHEET, "it's 翻譯 (v2)")
        .await
        .unwrap();
    assert_eq!(table, rows(&[&["key"]]));
}

#[tokio::test]
async fn read_tab_non_string_cells_are_stringified() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"values": [["k", 12, true, null]]})),
        )
        .mount(&server)
        .await;
    let table = client(&server).await.read_tab(SHEET, "t").await.unwrap();
    assert_eq!(table, rows(&[&["k", "12", "true", ""]]));
}

// ---------- write

#[tokio::test]
async fn write_tab_clears_then_updates_with_raw() {
    let server = MockServer::start().await;
    let table = rows(&[&["key", "en"], &["ok", "=SUM(1)"]]);

    Mock::given(method("POST"))
        .and(path(format!(
            "/v4/spreadsheets/{SHEET}/values/%27i18n%27:clear"
        )))
        .and(header("authorization", "Bearer tok-1"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"clearedRange": "'i18n'!A1:Z1000"})),
        )
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path(format!(
            "/v4/spreadsheets/{SHEET}/values/%27i18n%27%21A1"
        )))
        .and(query_param("valueInputOption", "RAW"))
        .and(body_json(json!({
            "range": "'i18n'!A1",
            "majorDimension": "ROWS",
            "values": [["key","en"],["ok","=SUM(1)"]]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"updatedCells": 4})))
        .expect(1)
        .mount(&server)
        .await;

    client(&server)
        .await
        .write_tab(SHEET, "i18n", &table)
        .await
        .unwrap();

    // clear must precede update
    let received: Vec<Request> = server.received_requests().await.unwrap();
    let methods: Vec<_> = received.iter().map(|r| r.method.to_string()).collect();
    assert_eq!(methods, ["POST", "PUT"]);
}

#[tokio::test]
async fn write_tab_header_only_is_fine() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(body_json(
            json!({"range": "'t'!A1", "majorDimension": "ROWS", "values": [["key","en"]]}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    client(&server)
        .await
        .write_tab(SHEET, "t", &rows(&[&["key", "en"]]))
        .await
        .unwrap();
}

#[tokio::test]
async fn write_tab_update_failure_after_clear_is_flagged() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .respond_with(google_error(503, "backend"))
        .mount(&server)
        .await;
    let err = client(&server)
        .await
        .write_tab(SHEET, "t", &rows(&[&["key"]]))
        .await
        .unwrap_err();
    assert_eq!(err.code(), "WRITE_AFTER_CLEAR_FAILED");
    match err {
        SheetsError::WriteAfterClearFailed { source } => {
            assert!(matches!(*source, SheetsError::ServerError { status: 503 }));
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[tokio::test]
async fn write_tab_clear_failure_is_not_flagged_as_after_clear() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(google_error(403, "The caller does not have permission"))
        .mount(&server)
        .await;
    let err = client(&server)
        .await
        .write_tab(SHEET, "t", &rows(&[&["key"]]))
        .await
        .unwrap_err();
    assert!(matches!(err, SheetsError::PermissionDenied { .. }));
}

// ---------- error classification

async fn read_err(status: u16, message: &str) -> SheetsError {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(google_error(status, message))
        .mount(&server)
        .await;
    client(&server)
        .await
        .read_tab(SHEET, "tab")
        .await
        .unwrap_err()
}

#[tokio::test]
async fn classifies_403_404_400_429_5xx_other() {
    let e = read_err(403, "The caller does not have permission").await;
    assert!(matches!(&e, SheetsError::PermissionDenied { sheet_id, .. } if sheet_id == SHEET));
    assert_eq!(e.code(), "PERMISSION_DENIED");

    let e = read_err(404, "Requested entity was not found.").await;
    assert!(matches!(&e, SheetsError::SheetNotFound { sheet_id } if sheet_id == SHEET));

    let e = read_err(400, "Unable to parse range: 'tab'").await;
    assert!(matches!(&e, SheetsError::TabNotFound { tab, .. } if tab == "tab"));
    assert!(e.to_string().contains("\"tab\""));

    let e = read_err(400, "Invalid JSON payload").await;
    assert!(
        matches!(&e, SheetsError::Http { status: 400, message } if message == "Invalid JSON payload")
    );

    let e = read_err(429, "Quota exceeded").await;
    assert!(matches!(e, SheetsError::RateLimited));
    assert!(e.is_transient());

    let e = read_err(502, "Bad gateway").await;
    assert!(matches!(e, SheetsError::ServerError { status: 502 }));
    assert!(e.is_transient());
}

#[tokio::test]
async fn permission_error_mentions_service_account_email() {
    #[derive(Debug)]
    struct Sa;
    #[async_trait::async_trait]
    impl TokenProvider for Sa {
        async fn token(&self) -> Result<String, SheetsError> {
            Ok("t".into())
        }
        async fn invalidate(&self) {}
        fn service_account_email(&self) -> Option<String> {
            Some("bot@proj.iam.gserviceaccount.com".into())
        }
    }
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(google_error(403, "no"))
        .mount(&server)
        .await;
    let c = SheetsClient::builder(Credentials::ApplicationDefault)
        .base_url(server.uri())
        .token_provider(Arc::new(Sa))
        .build()
        .await
        .unwrap();
    let err = c.read_tab(SHEET, "t").await.unwrap_err();
    assert!(
        err.to_string()
            .contains("share the sheet with bot@proj.iam.gserviceaccount.com")
    );
}

#[tokio::test]
async fn connection_refused_is_network_error() {
    // Port 1 is reserved and nothing listens there.
    let c = SheetsClient::builder(Credentials::AccessToken("t".into()))
        .base_url("http://127.0.0.1:1")
        .build()
        .await
        .unwrap();
    let err = c.read_tab(SHEET, "t").await.unwrap_err();
    assert!(matches!(err, SheetsError::Network(_)));
    assert!(err.is_transient());
}

#[tokio::test]
async fn unauthorized_once_invalidates_and_retries() {
    #[derive(Debug)]
    struct Rotating(AtomicUsize);
    #[async_trait::async_trait]
    impl TokenProvider for Rotating {
        async fn token(&self) -> Result<String, SheetsError> {
            Ok(format!("tok-{}", self.0.load(Ordering::SeqCst)))
        }
        async fn invalidate(&self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header("authorization", "Bearer tok-0"))
        .respond_with(google_error(401, "expired"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(header("authorization", "Bearer tok-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"values": [["key"]]})))
        .expect(1)
        .mount(&server)
        .await;
    let provider = Arc::new(Rotating(AtomicUsize::new(0)));
    let c = SheetsClient::builder(Credentials::ApplicationDefault)
        .base_url(server.uri())
        .token_provider(provider.clone())
        .build()
        .await
        .unwrap();
    assert_eq!(c.read_tab(SHEET, "t").await.unwrap(), rows(&[&["key"]]));
}

#[tokio::test]
async fn unauthorized_twice_is_auth_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(google_error(401, "bad token"))
        .expect(2)
        .mount(&server)
        .await;
    let err = client(&server)
        .await
        .read_tab(SHEET, "t")
        .await
        .unwrap_err();
    assert!(matches!(err, SheetsError::Auth(m) if m == "bad token"));
}

// ---------- live (manual)

/// Run with:
/// GSLM_TEST_SHEET_ID=... GSLM_TEST_TAB=... GOOGLE_APPLICATION_CREDENTIALS=... \
///   cargo test -p gslm-sheets -- --ignored live_round_trip
#[tokio::test]
#[ignore]
async fn live_round_trip() {
    let sheet = std::env::var("GSLM_TEST_SHEET_ID").expect("GSLM_TEST_SHEET_ID");
    let tab = std::env::var("GSLM_TEST_TAB").expect("GSLM_TEST_TAB");
    let c = SheetsClient::builder(Credentials::ApplicationDefault)
        .build()
        .await
        .unwrap();
    let table = rows(&[
        &["key", "en", "zh-TW"],
        &["ok", "OK", "好"],
        &["formula", "=1+1", ""],
    ]);
    c.write_tab(&sheet, &tab, &table).await.unwrap();
    let back = c.read_tab(&sheet, &tab).await.unwrap();
    assert_eq!(
        back,
        rows(&[
            &["key", "en", "zh-TW"],
            &["ok", "OK", "好"],
            &["formula", "=1+1"]
        ])
    );
}

// ---------- timeouts

#[tokio::test]
async fn request_timeout_is_configurable_and_reported_as_network_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "values": [] }))
                .set_delay(std::time::Duration::from_millis(500)),
        )
        .mount(&server)
        .await;

    let c = SheetsClient::builder(Credentials::AccessToken("tok-1".into()))
        .base_url(server.uri())
        .timeout(std::time::Duration::from_millis(50))
        .build()
        .await
        .unwrap();
    let err = c.read_tab(SHEET, "i18n").await.unwrap_err();
    assert!(matches!(err, SheetsError::Network(_)), "{err:?}");
}
