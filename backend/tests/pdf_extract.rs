//! `POST /api/pdf/extract` — same feature gate as `/api/ocr` (both need
//! `OCR_LLM_API_KEY`), so the test harness sees it disabled. A real
//! extraction needs network + a live key; `application::ocr::tests` covers
//! the response-parsing logic, this just proves the route is wired.
#![allow(clippy::unwrap_used)]

mod common;

use common::TestApp;

/// A minimal PDF with real extractable text (built the same way the unit
/// tests build fixtures, just inlined here to avoid depending on a
/// `#[cfg(test)]`-only helper from another crate module).
fn tiny_pdf_with_text() -> Vec<u8> {
    // Plain, valid-enough PDF/1.4 body: SOI-free format, no test dependency —
    // reuse a pre-baked minimal PDF, one page, no text needed since the
    // request never reaches pdf::text (the 503 gate fires first).
    b"%PDF-1.4\n1 0 obj<<>>endobj\ntrailer<<>>\n%%EOF".to_vec()
}

#[tokio::test]
async fn extract_is_unavailable_without_configuration() {
    let mut app = TestApp::new();
    let res = app
        .post_file_key(
            "/api/pdf/extract",
            "test-key",
            "file",
            "invoice.pdf",
            "application/pdf",
            &tiny_pdf_with_text(),
        )
        .await;
    assert_eq!(res.status, 503);
    assert!(res.text().to_lowercase().contains("not configured"));
}

#[tokio::test]
async fn extract_requires_authentication() {
    let mut app = TestApp::new();
    let res = app
        .post_file(
            "/api/pdf/extract",
            "file",
            "invoice.pdf",
            "application/pdf",
            &tiny_pdf_with_text(),
        )
        .await;
    assert_eq!(res.status, 401);
}
