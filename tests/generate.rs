#![allow(clippy::unwrap_used)]
mod common;

use axum::http::{StatusCode, header};
use common::{TestApp, xlsx_bytes};
use serde_json::json;

const KEY: &str = "test-key";
const OCTET: &str = "application/octet-stream";
const ZIP_MAGIC: &[u8] = b"PK\x03\x04";

#[tokio::test]
async fn generate_xlsx_from_object_rows() {
    let mut app = TestApp::new();
    let r = app
        .post_json_key(
            "/api/generate/xlsx",
            KEY,
            json!({
                "rows": [
                    { "Name": "Ana", "Age": 34, "Active": true },
                    { "Name": "Luis", "Age": 28, "Active": false }
                ],
                "options": { "sheet_name": "Personas" }
            }),
        )
        .await;

    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(
        r.headers.get(header::CONTENT_TYPE).unwrap(),
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    );
    assert!(
        r.headers
            .get(header::CONTENT_DISPOSITION)
            .unwrap()
            .to_str()
            .unwrap()
            .contains("Personas.xlsx")
    );
    assert!(r.body.starts_with(ZIP_MAGIC), "xlsx is a zip archive");
}

#[tokio::test]
async fn generate_xlsx_from_array_rows_with_columns() {
    let mut app = TestApp::new();
    let r = app
        .post_json_key(
            "/api/generate/xlsx",
            KEY,
            json!({ "columns": ["a", "b"], "rows": [[1, 2], [3, 4]] }),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.body.starts_with(ZIP_MAGIC));
}

/// The round trip: xlsx → parse → xlsx again via `convert?to=xlsx`.
#[tokio::test]
async fn convert_can_target_xlsx() {
    let mut app = TestApp::new();
    let r = app
        .post_file_key(
            "/api/process/convert?to=xlsx",
            KEY,
            "file",
            "ventas.xlsx",
            OCTET,
            &xlsx_bytes(),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(
        r.headers.get(header::CONTENT_TYPE).unwrap(),
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    );
    assert!(r.body.starts_with(ZIP_MAGIC));
}
