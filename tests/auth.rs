#![allow(clippy::unwrap_used)]
mod common;

use axum::http::StatusCode;
use common::TestApp;

#[tokio::test]
async fn config_and_csrf_are_public() {
    let mut app = TestApp::new();

    let cfg = app.get("/api/v1/config").await;
    assert_eq!(cfg.status, StatusCode::OK);
    assert_eq!(cfg.json["app_name"], "Filers Test");

    assert_eq!(
        app.get("/api/v1/csrf/").await.status,
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn me_requires_a_session() {
    let mut app = TestApp::new();
    assert_eq!(
        app.get("/api/v1/auth/me").await.status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn login_rejects_bad_credentials() {
    let mut app = TestApp::new();

    let r = app.login("admin@filers.test", "wrong-password").await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);

    let r = app.login("ghost@filers.test", "whatever12345").await;
    assert_eq!(r.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn full_login_me_logout_cycle() {
    let mut app = TestApp::new();

    let login = app.login_admin().await;
    assert_eq!(login.status, StatusCode::OK);
    assert_eq!(login.json["user"]["email"], "admin@filers.test");
    assert_eq!(login.json["roles"][0], "admin");
    assert!(
        login
            .set_cookie
            .as_deref()
            .is_some_and(|c| c.contains("HttpOnly") && c.contains("SameSite=Lax"))
    );

    let me = app.get("/api/v1/auth/me").await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.json["user"]["email"], "admin@filers.test");

    assert_eq!(
        app.post_empty("/api/v1/auth/logout").await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        app.get("/api/v1/auth/me").await.status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn login_is_rate_limited_per_ip() {
    let mut app = TestApp::new();
    // test_config sets AUTH_RATE_LIMIT to 5/60.
    for _ in 0..5 {
        let s = app.login("admin@filers.test", "nope").await.status;
        assert_eq!(s, StatusCode::UNAUTHORIZED);
    }
    // 6th attempt (even with the *right* password) is throttled.
    assert_eq!(
        app.login_admin().await.status,
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn refresh_endpoint_always_401() {
    let mut app = TestApp::new();
    assert_eq!(
        app.post_empty("/api/v1/auth/refresh").await.status,
        StatusCode::UNAUTHORIZED
    );
}

/// Suspending (or changing the password of) a user must drop their live session
/// immediately, not wait for the 7-day TTL.
#[tokio::test]
async fn suspending_a_user_revokes_their_session() {
    let mut app = TestApp::new();

    app.login_user().await; // maria, role "user"
    assert!(app.get("/api/v1/auth/me").await.ok());
    let maria_cookie = app.snapshot_cookie();

    app.login_admin().await;
    let users = app.get("/api/v1/users").await;
    let maria_id = users.json["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|u| u["email"] == "maria@filers.test")
        .and_then(|u| u["id"].as_str())
        .unwrap()
        .to_string();

    let updated = app
        .put_json(
            &format!("/api/v1/users/{maria_id}"),
            serde_json::json!({ "status": "suspended" }),
        )
        .await;
    assert_eq!(updated.status, StatusCode::OK);

    // Maria's previously-valid cookie is now dead.
    app.restore_cookie(maria_cookie);
    assert_eq!(
        app.get("/api/v1/auth/me").await.status,
        StatusCode::UNAUTHORIZED
    );
}
