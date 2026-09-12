//! `POST /api/ocr` — disabled (503) in the test harness, which runs without
//! `OCR_LLM_API_KEY`. A real OCR round-trip would need network + a live key,
//! so the unit tests in `application::ocr` cover the request/response
//! shaping; this just proves the endpoint is wired and fails closed.
#![allow(clippy::unwrap_used)]

mod common;

use common::TestApp;

const TINY_PNG: &[u8] = &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];

#[tokio::test]
async fn ocr_is_unavailable_without_configuration() {
    let mut app = TestApp::new();
    let res = app
        .post_file_key(
            "/api/ocr",
            "test-key",
            "file",
            "receipt.png",
            "image/png",
            TINY_PNG,
        )
        .await;
    assert_eq!(res.status, 503);
    assert!(res.text().to_lowercase().contains("not configured"));
}

#[tokio::test]
async fn ocr_requires_authentication() {
    let mut app = TestApp::new();
    let res = app
        .post_file("/api/ocr", "file", "receipt.png", "image/png", TINY_PNG)
        .await;
    assert_eq!(res.status, 401);
}
