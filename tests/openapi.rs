//! The OpenAPI spec + Scalar docs UI, gated by `Config::enable_api_docs`.
#![allow(clippy::unwrap_used)]

mod common;

use common::TestApp;

#[tokio::test]
async fn openapi_json_is_served_and_describes_the_public_api() {
    // `test_config` sets enable_api_docs = true.
    let mut app = TestApp::new();

    let res = app.get("/api/openapi.json").await;
    assert_eq!(res.status, 200);

    let paths = res.json["paths"].as_object().expect("paths object");
    assert!(paths.contains_key("/api/process"));
    assert!(paths.contains_key("/api/process/pipeline"));
    assert!(paths.contains_key("/api/ext/auth/token"));
    // Admin-only routes are intentionally excluded.
    assert!(!paths.contains_key("/api/v1/users"));

    let schemes = &res.json["components"]["securitySchemes"];
    assert!(schemes["api_key"].is_object());
    assert!(schemes["bearer"].is_object());
}

#[tokio::test]
async fn scalar_ui_is_served() {
    let mut app = TestApp::new();
    let res = app.get("/api/docs").await;
    assert_eq!(res.status, 200);
    assert!(res.text().to_lowercase().contains("scalar"));
}

#[tokio::test]
async fn docs_are_absent_when_disabled() {
    let tmp = tempfile::tempdir().unwrap();
    let mut config = rust_api::bootstrap::test_config(tmp.path().to_string_lossy().to_string());
    config.enable_api_docs = false;
    let router = rust_api::bootstrap::build_app(config);

    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    let res = router
        .oneshot(
            Request::builder()
                .uri("/api/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), 404);
}
