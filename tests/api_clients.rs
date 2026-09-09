#![allow(clippy::unwrap_used)]
mod common;

use axum::http::StatusCode;
use common::{TestApp, csv_bytes};
use serde_json::json;

async fn new_client(app: &mut TestApp, scopes: serde_json::Value) -> (String, String, String) {
    let r = app
        .post_json(
            "/api/v1/api-clients",
            json!({ "name": "ERP", "scopes": scopes }),
        )
        .await;
    assert_eq!(r.status, StatusCode::CREATED);
    (
        r.json["client"]["id"].as_str().unwrap().to_string(),
        r.json["client"]["client_id"].as_str().unwrap().to_string(),
        r.json["secret"].as_str().unwrap().to_string(),
    )
}

async fn token(app: &mut TestApp, client_id: &str, secret: &str) -> Resp2 {
    let r = app
        .post_json(
            "/api/ext/auth/token",
            json!({ "client_id": client_id, "client_secret": secret }),
        )
        .await;
    Resp2 {
        status: r.status,
        access: r.json["access_token"].as_str().map(str::to_string),
        refresh: r.json["refresh_token"].as_str().map(str::to_string),
    }
}

struct Resp2 {
    status: StatusCode,
    access: Option<String>,
    refresh: Option<String>,
}

#[tokio::test]
async fn admin_manages_clients_only_with_session() {
    let mut app = TestApp::new();
    assert_eq!(
        app.get("/api/v1/api-clients").await.status,
        StatusCode::UNAUTHORIZED
    );
    app.login_user().await;
    assert_eq!(
        app.get("/api/v1/api-clients").await.status,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn client_credentials_flow() {
    let mut app = TestApp::new();
    app.login_admin().await;
    let (_id, client_id, secret) = new_client(&mut app, json!(["files:write", "files:read"])).await;
    app.logout_local();

    // wrong secret
    assert_eq!(
        token(&mut app, &client_id, "nope").await.status,
        StatusCode::UNAUTHORIZED
    );

    let grant = token(&mut app, &client_id, &secret).await;
    assert_eq!(grant.status, StatusCode::OK);
    let access = grant.access.unwrap();

    // Bearer call to the processing API works
    let r = app
        .post_file_bearer(
            "/api/process",
            &access,
            "file",
            "c.csv",
            "text/csv",
            &csv_bytes(),
        )
        .await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.json["format"], "csv");

    // bad bearer → 401
    assert_eq!(
        app.post_file_bearer(
            "/api/process",
            "deadbeef",
            "file",
            "c.csv",
            "text/csv",
            &csv_bytes()
        )
        .await
        .status,
        StatusCode::UNAUTHORIZED
    );

    // refresh is single-use
    let refresh = grant.refresh.unwrap();
    let first = app
        .post_json("/api/ext/auth/refresh", json!({ "refresh_token": refresh }))
        .await;
    assert_eq!(first.status, StatusCode::OK);
    let rotated_access = first.json["access_token"].as_str().unwrap().to_string();
    let rotated_refresh = first.json["refresh_token"].as_str().unwrap().to_string();

    // Replaying the spent refresh token is treated as theft: 401 …
    let reused = app
        .post_json("/api/ext/auth/refresh", json!({ "refresh_token": refresh }))
        .await;
    assert_eq!(reused.status, StatusCode::UNAUTHORIZED);

    // … and the whole token family is revoked — the access + refresh that the
    // legitimate rotation just produced no longer work.
    assert_eq!(
        app.get_bearer(
            "/api/jobs/00000000-0000-0000-0000-000000000000",
            &rotated_access
        )
        .await
        .status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.post_json(
            "/api/ext/auth/refresh",
            json!({ "refresh_token": rotated_refresh })
        )
        .await
        .status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn a_client_cannot_read_another_clients_job() {
    let mut app = TestApp::new();
    app.login_admin().await;
    let (_a, a_cid, a_sec) = new_client(&mut app, json!(["*"])).await;
    let (_b, b_cid, b_sec) = new_client(&mut app, json!(["*"])).await;
    app.logout_local();

    let a_access = token(&mut app, &a_cid, &a_sec).await.access.unwrap();
    let b_access = token(&mut app, &b_cid, &b_sec).await.access.unwrap();

    // Client A starts a batch job (the base dir is empty — it just completes).
    let started = app
        .json_with(
            "POST",
            "/api/process/batch",
            &[("authorization", &format!("Bearer {a_access}"))],
            json!({}),
        )
        .await;
    assert_eq!(started.status, StatusCode::OK);
    let job_id = started.json["job_id"].as_str().unwrap().to_string();

    // A can read its own job; B gets a 404 (not 403 — no existence oracle).
    assert_eq!(
        app.get_bearer(&format!("/api/jobs/{job_id}"), &a_access)
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        app.get_bearer(&format!("/api/jobs/{job_id}"), &b_access)
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn scope_is_enforced() {
    let mut app = TestApp::new();
    app.login_admin().await;
    let (_id, client_id, secret) = new_client(&mut app, json!(["files:read"])).await;
    app.logout_local();
    let access = token(&mut app, &client_id, &secret).await.access.unwrap();

    // files:read cannot write
    assert_eq!(
        app.post_file_bearer(
            "/api/process",
            &access,
            "file",
            "c.csv",
            "text/csv",
            &csv_bytes()
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn monthly_quota_returns_429() {
    let mut app = TestApp::new();
    app.login_admin().await;
    let (id, client_id, secret) = new_client(&mut app, json!(["*"])).await;
    app.patch_json(
        &format!("/api/v1/api-clients/{id}"),
        json!({ "monthly_page_quota": 2 }),
    )
    .await;
    app.logout_local();
    let access = token(&mut app, &client_id, &secret).await.access.unwrap();

    for _ in 0..2 {
        let s = app
            .post_file_bearer(
                "/api/process",
                &access,
                "file",
                "c.csv",
                "text/csv",
                &csv_bytes(),
            )
            .await
            .status;
        assert_eq!(s, StatusCode::OK);
    }
    let over = app
        .post_file_bearer(
            "/api/process",
            &access,
            "file",
            "c.csv",
            "text/csv",
            &csv_bytes(),
        )
        .await;
    assert_eq!(over.status, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn rotate_invalidates_tokens_and_revoke_soft_deletes() {
    let mut app = TestApp::new();
    app.login_admin().await;
    let (id, client_id, secret) = new_client(&mut app, json!(["*"])).await;
    let access = {
        app.logout_local();
        let a = token(&mut app, &client_id, &secret).await.access.unwrap();
        app.login_admin().await;
        a
    };

    let rotated = app
        .post_empty(&format!("/api/v1/api-clients/{id}/rotate"))
        .await;
    assert_eq!(rotated.status, StatusCode::OK);

    // old access token now rejected
    app.logout_local();
    assert_eq!(
        app.post_file_bearer(
            "/api/process",
            &access,
            "file",
            "c.csv",
            "text/csv",
            &csv_bytes()
        )
        .await
        .status,
        StatusCode::UNAUTHORIZED
    );

    app.login_admin().await;
    assert_eq!(
        app.delete(&format!("/api/v1/api-clients/{id}"))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    let list = app.get("/api/v1/api-clients").await;
    assert_eq!(list.json[0]["active"], false);
}
