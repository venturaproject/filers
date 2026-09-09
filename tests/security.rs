#![allow(clippy::unwrap_used)]
mod common;

use axum::http::StatusCode;
use common::TestApp;
use serde_json::json;

#[tokio::test]
async fn security_headers_on_every_response() {
    let mut app = TestApp::new();
    let r = app.get("/api/v1/config").await;
    for h in [
        "x-content-type-options",
        "x-frame-options",
        "referrer-policy",
        "cross-origin-opener-policy",
        "x-permitted-cross-domain-policies",
    ] {
        assert!(r.headers.contains_key(h), "missing {h}");
    }
    assert_eq!(r.headers["x-content-type-options"], "nosniff");
    assert_eq!(r.headers["x-frame-options"], "DENY");
    // not HTTPS in tests → no HSTS
    assert!(!r.headers.contains_key("strict-transport-security"));
}

#[tokio::test]
async fn batch_path_traversal_is_blocked() {
    let mut app = TestApp::new();

    // `/api/process/batch` authenticates with x-api-key, not the session.
    for bad in ["../src", "..", "../../etc", "/etc"] {
        let r = app
            .post_json_key("/api/process/batch", "test-key", json!({ "path": bad }))
            .await;
        assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY, "path {bad}");
        assert!(
            r.json["error"]
                .as_str()
                .unwrap()
                .contains("relative to the configured base dir")
        );
    }

    // no `path` → scans the (empty) base dir, succeeds with a 0-file job
    let ok = app
        .post_json_key("/api/process/batch", "test-key", json!({}))
        .await;
    assert_eq!(ok.status, StatusCode::OK);
}

#[tokio::test]
async fn invalid_api_key_is_rejected() {
    let mut app = TestApp::new();
    let r = app
        .post_file_key(
            "/api/process",
            "not-a-real-key",
            "file",
            "c.csv",
            "text/csv",
            b"a,b\n1,2\n",
        )
        .await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
}
