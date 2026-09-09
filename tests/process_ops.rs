#![allow(clippy::unwrap_used)]
mod common;

use axum::http::{StatusCode, header};
use common::{TestApp, xlsx_bytes};

const KEY: &str = "test-key";
const OCTET: &str = "application/octet-stream";

/// A small CSV: 3 columns, one bad email, one out-of-range age, a duplicate id.
fn people_csv() -> &'static [u8] {
    b"id,email,age\n\
      1,ana@example.com,34\n\
      2,not-an-email,29\n\
      1,luis@example.com,200\n"
}

#[tokio::test]
async fn profile_infers_types_and_counts() {
    let mut app = TestApp::new();
    let r = app
        .post_file_key(
            "/api/process/profile",
            KEY,
            "file",
            "p.csv",
            OCTET,
            people_csv(),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);

    let cols = r.json["report"]["profile"].as_array().unwrap();
    let by = |name: &str| cols.iter().find(|c| c["column"] == name).unwrap().clone();

    assert_eq!(by("age")["inferred_type"], "integer");
    assert_eq!(by("email")["inferred_type"], "string");
    assert_eq!(by("id")["distinct"], 2); // 1 appears twice
    assert_eq!(r.json["stats"]["total_rows"], 3);
}

#[tokio::test]
async fn validate_reports_rule_violations() {
    let mut app = TestApp::new();
    let schema = serde_json::json!({
        "columns": {
            "id":    { "required": true, "unique": true },
            "email": { "regex": "^[^@\\s]+@[^@\\s]+$" },
            "age":   { "type": "integer", "min": 0, "max": 120 }
        }
    })
    .to_string();

    let r = app
        .post_files_key(
            "/api/process/validate",
            KEY,
            &[
                (
                    "schema",
                    "schema.json",
                    "application/json",
                    schema.as_bytes(),
                ),
                ("file", "p.csv", OCTET, people_csv()),
            ],
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);

    let report = &r.json["report"];
    assert_eq!(report["valid"], false);
    let errors = report["errors"].as_array().unwrap();
    // one bad email (regex), one age over max, one duplicate id
    assert!(
        errors
            .iter()
            .any(|e| e["column"] == "email" && e["rule"] == "regex")
    );
    assert!(
        errors
            .iter()
            .any(|e| e["column"] == "age" && e["rule"] == "max")
    );
    assert!(
        errors
            .iter()
            .any(|e| e["column"] == "id" && e["rule"] == "unique")
    );
}

#[tokio::test]
async fn validate_passes_a_clean_file() {
    let mut app = TestApp::new();
    let schema = serde_json::json!({ "columns": { "id": { "required": true } } }).to_string();
    let r = app
        .post_files_key(
            "/api/process/validate",
            KEY,
            &[
                ("schema", "s.json", "application/json", schema.as_bytes()),
                ("file", "p.csv", OCTET, b"id\n1\n2\n3\n"),
            ],
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["report"]["valid"], true);
}

#[tokio::test]
async fn convert_xlsx_to_csv_and_ndjson() {
    let mut app = TestApp::new();

    let csv = app
        .post_file_key(
            "/api/process/convert?to=csv",
            KEY,
            "file",
            "ventas.xlsx",
            OCTET,
            &xlsx_bytes(),
        )
        .await;
    assert_eq!(csv.status, StatusCode::OK);
    assert_eq!(
        csv.headers.get(header::CONTENT_TYPE).unwrap(),
        "text/csv; charset=utf-8"
    );
    assert!(
        csv.headers
            .get(header::CONTENT_DISPOSITION)
            .unwrap()
            .to_str()
            .unwrap()
            .contains("ventas.csv")
    );
    assert!(
        csv.text().lines().count() >= 2,
        "header + at least one data row"
    );

    let nd = app
        .post_file_key(
            "/api/process/convert?to=ndjson",
            KEY,
            "file",
            "ventas.xlsx",
            OCTET,
            &xlsx_bytes(),
        )
        .await;
    assert_eq!(nd.status, StatusCode::OK);
    assert_eq!(
        nd.headers.get(header::CONTENT_TYPE).unwrap(),
        "application/x-ndjson"
    );
}

#[tokio::test]
async fn convert_rejects_unknown_target() {
    let mut app = TestApp::new();
    let r = app
        .post_file_key(
            "/api/process/convert?to=parquet",
            KEY,
            "file",
            "v.xlsx",
            OCTET,
            &xlsx_bytes(),
        )
        .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn ops_leave_a_trace_with_the_operation_name() {
    let mut app = TestApp::new();
    app.post_file_key(
        "/api/process/profile",
        KEY,
        "file",
        "p.csv",
        OCTET,
        people_csv(),
    )
    .await;

    app.login_admin().await;
    let jobs = app.get("/api/v1/jobs").await;
    let ops: Vec<&str> = jobs.json["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|j| j["operation"].as_str().unwrap())
        .collect();
    assert!(ops.contains(&"profile"));
}
