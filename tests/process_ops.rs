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
async fn transform_filters_selects_and_renames() {
    let mut app = TestApp::new();
    let spec = serde_json::json!({
        "select": ["id", "age"],
        "rename": { "age": "years" },
        "cast": { "age": "integer" },
        "filter": { "age": { "gte": 30 } }
    })
    .to_string();

    let r = app
        .post_files_key(
            "/api/process/transform",
            KEY,
            &[
                ("spec", "spec.json", "application/json", spec.as_bytes()),
                ("file", "p.csv", OCTET, people_csv()),
            ],
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["columns"], serde_json::json!(["id", "years"]));
    // ages: 34, 29, 200 → keep 34 and 200
    assert_eq!(r.json["stats"]["returned_rows"], 2);
    assert_eq!(r.json["matched_rows"], 2);
}

#[tokio::test]
async fn transform_can_return_a_csv_download() {
    let mut app = TestApp::new();
    let spec = serde_json::json!({ "select": ["id"] }).to_string();
    let r = app
        .post_files_key(
            "/api/process/transform?to=csv",
            KEY,
            &[
                ("spec", "s.json", "application/json", spec.as_bytes()),
                ("file", "p.csv", OCTET, people_csv()),
            ],
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(
        r.headers.get(header::CONTENT_TYPE).unwrap(),
        "text/csv; charset=utf-8"
    );
    assert_eq!(r.text().lines().next().unwrap().trim(), "id");
}

#[tokio::test]
async fn diff_detects_added_removed_and_changed_rows() {
    let mut app = TestApp::new();
    let a = b"id,status\n1,new\n2,new\n3,done\n";
    let b = b"id,status\n1,new\n2,done\n4,new\n";

    let r = app
        .post_files_key(
            "/api/process/diff?key=id",
            KEY,
            &[("a", "a.csv", OCTET, a), ("b", "b.csv", OCTET, b)],
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);

    let s = &r.json["report"]["summary"];
    assert_eq!(s["added"], 1); // id 4
    assert_eq!(s["removed"], 1); // id 3
    assert_eq!(s["changed"], 1); // id 2: new → done
    assert_eq!(s["unchanged"], 1); // id 1

    let changed = r.json["report"]["changed"].as_array().unwrap();
    assert_eq!(changed[0]["key"]["id"], serde_json::json!(2));
    assert_eq!(changed[0]["changes"]["status"]["from"], "new");
    assert_eq!(changed[0]["changes"]["status"]["to"], "done");
}

#[tokio::test]
async fn diff_requires_a_key() {
    let mut app = TestApp::new();
    let r = app
        .post_files_key(
            "/api/process/diff",
            KEY,
            &[
                ("a", "a.csv", OCTET, b"id\n1\n"),
                ("b", "b.csv", OCTET, b"id\n1\n"),
            ],
        )
        .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn pipeline_validates_then_transforms_then_converts() {
    let mut app = TestApp::new();
    let pipeline = serde_json::json!({
        "steps": [
            { "op": "validate", "schema": { "columns": { "id": { "required": true } } } },
            { "op": "transform", "spec": { "select": ["id", "age"], "filter": { "age": { "gte": 30 } } } },
            { "op": "convert", "to": "csv" }
        ]
    })
    .to_string();

    let r = app
        .post_files_key(
            "/api/process/pipeline",
            KEY,
            &[
                (
                    "pipeline",
                    "p.json",
                    "application/json",
                    pipeline.as_bytes(),
                ),
                ("file", "p.csv", OCTET, people_csv()),
            ],
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(
        r.headers.get(header::CONTENT_TYPE).unwrap(),
        "text/csv; charset=utf-8"
    );
    let text = r.text();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[0].trim(), "id,age");
    assert_eq!(lines.len(), 3); // header + ages 34 and 200
}

#[tokio::test]
async fn pipeline_stops_with_422_on_validation_failure() {
    let mut app = TestApp::new();
    let pipeline = serde_json::json!({
        "steps": [
            { "op": "validate", "schema": { "columns": { "email": { "regex": "^\\S+@\\S+$" } } } },
            { "op": "convert", "to": "json" }
        ]
    })
    .to_string();

    let r = app
        .post_files_key(
            "/api/process/pipeline",
            KEY,
            &[
                (
                    "pipeline",
                    "p.json",
                    "application/json",
                    pipeline.as_bytes(),
                ),
                ("file", "p.csv", OCTET, people_csv()), // has "not-an-email"
            ],
        )
        .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(r.json["error"], "validation failed");
    let steps = r.json["steps"].as_array().unwrap();
    assert_eq!(steps[0]["op"], "validate");
    assert_eq!(steps[0]["valid"], false);
}

#[tokio::test]
async fn pipeline_without_a_terminal_step_returns_json() {
    let mut app = TestApp::new();
    let pipeline =
        serde_json::json!({ "steps": [ { "op": "profile" }, { "op": "transform", "spec": { "limit": 1 } } ] })
            .to_string();
    let r = app
        .post_files_key(
            "/api/process/pipeline",
            KEY,
            &[
                (
                    "pipeline",
                    "p.json",
                    "application/json",
                    pipeline.as_bytes(),
                ),
                ("file", "p.csv", OCTET, people_csv()),
            ],
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["stats"]["returned_rows"], 1);
    let ops: Vec<&str> = r.json["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["op"].as_str().unwrap())
        .collect();
    assert_eq!(ops, ["profile", "transform"]);
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
