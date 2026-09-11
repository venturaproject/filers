//! Liveness (`/health`) and readiness (`/health/ready`) probes.
#![allow(clippy::unwrap_used)]

mod common;

use common::TestApp;

#[tokio::test]
async fn liveness_is_ok() {
    let mut app = TestApp::new();
    let res = app.get("/health").await;
    assert_eq!(res.status, 200);
    assert_eq!(res.text().trim(), "ok");
}

#[tokio::test]
async fn readiness_is_ok_without_a_database() {
    // The test harness runs the in-memory configuration (no DATABASE_URL), so
    // there is nothing external to check — readiness is unconditionally true.
    let mut app = TestApp::new();
    let res = app.get("/health/ready").await;
    assert_eq!(res.status, 200);
    assert_eq!(res.json["status"], "ready");
}
