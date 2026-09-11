#![allow(clippy::unwrap_used)]
mod common;

use axum::http::StatusCode;
use common::{TestApp, csv_bytes, xlsx_bytes};

const OCTET: &str = "application/octet-stream";

async fn key_upload(app: &mut TestApp, path: &str, filename: &str, bytes: &[u8]) -> common::Resp {
    app.post_file_key(path, "test-key", "file", filename, OCTET, bytes)
        .await
}

#[tokio::test]
async fn processing_requires_a_key() {
    let mut app = TestApp::new();
    let r = app
        .post_files(
            "/api/process",
            &[("file", "c.csv", "text/csv", &csv_bytes())],
        )
        .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn parses_csv_with_service_key() {
    let mut app = TestApp::new();
    let r = key_upload(&mut app, "/api/process?max_rows=1", "c.csv", &csv_bytes()).await;

    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["format"], "csv");
    assert_eq!(
        r.json["columns"],
        serde_json::json!(["name", "age", "city"])
    );
    assert_eq!(r.json["stats"]["total_rows"], 2);
    assert_eq!(r.json["stats"]["returned_rows"], 1);
}

#[tokio::test]
async fn parses_xlsx() {
    let mut app = TestApp::new();
    let r = key_upload(&mut app, "/api/process", "v.xlsx", &xlsx_bytes()).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["format"], "xlsx");
    assert!(r.json["stats"]["total_rows"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn rejects_unsupported_format() {
    let mut app = TestApp::new();
    let r = key_upload(&mut app, "/api/process", "notes.txt", b"hello").await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(r.json["error"].as_str().unwrap().contains("Unsupported"));
}

#[tokio::test]
async fn rejects_content_that_does_not_match_the_extension() {
    let mut app = TestApp::new();
    // A `.xlsx` that is not a ZIP archive — never handed to calamine.
    let r = key_upload(
        &mut app,
        "/api/process",
        "fake.xlsx",
        b"this is not a spreadsheet",
    )
    .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        r.json["error"]
            .as_str()
            .unwrap()
            .contains("does not look like")
    );
}

#[tokio::test]
async fn oversized_upload_is_rejected() {
    let mut app = TestApp::new();
    // test_config caps at 5 MB.
    let big = vec![b'a'; 6 * 1024 * 1024];
    let r = key_upload(&mut app, "/api/process", "big.csv", &big).await;
    assert!(
        r.status == StatusCode::UNPROCESSABLE_ENTITY || r.status == StatusCode::PAYLOAD_TOO_LARGE,
        "got {}",
        r.status
    );
}

#[tokio::test]
async fn every_call_leaves_a_trace() {
    let mut app = TestApp::new();

    key_upload(&mut app, "/api/process", "ok.csv", &csv_bytes()).await;
    key_upload(&mut app, "/api/process", "bad.txt", b"x").await;

    app.login_admin().await;
    let jobs = app.get("/api/v1/jobs").await;
    assert_eq!(jobs.status, StatusCode::OK);
    assert_eq!(jobs.json["total"], 2);

    let rows = jobs.json["data"].as_array().unwrap();
    assert!(
        rows.iter()
            .all(|j| j["kind"] == "sync" && j["origin"] == "api_key")
    );
    let statuses: Vec<&str> = rows.iter().map(|j| j["status"].as_str().unwrap()).collect();
    assert!(statuses.contains(&"completed") && statuses.contains(&"failed"));
}
