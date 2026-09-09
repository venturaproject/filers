#![allow(clippy::unwrap_used)]
mod common;

use axum::http::StatusCode;
use common::TestApp;
use serde_json::json;

#[tokio::test]
async fn admin_endpoints_require_a_session() {
    let mut app = TestApp::new();
    for path in ["/api/v1/users", "/api/v1/roles", "/api/v1/permissions"] {
        assert_eq!(
            app.get(path).await.status,
            StatusCode::UNAUTHORIZED,
            "{path}"
        );
    }
}

#[tokio::test]
async fn non_admin_user_is_forbidden() {
    let mut app = TestApp::new();
    app.login_user().await; // role "user"
    assert_eq!(app.get("/api/v1/users").await.status, StatusCode::FORBIDDEN);
    assert_eq!(app.get("/api/v1/roles").await.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn seeded_catalogue_is_listed() {
    let mut app = TestApp::new();
    app.login_admin().await;

    let users = app.get("/api/v1/users").await;
    assert_eq!(users.status, StatusCode::OK);
    assert_eq!(users.json["total"], 3);
    assert_eq!(users.json["stats"]["activos"], 3);

    let roles = app.get("/api/v1/roles").await;
    let names: Vec<&str> = roles.json["data"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"admin") && names.contains(&"user"));

    assert!(
        app.get("/api/v1/permissions").await.json["total"]
            .as_u64()
            .unwrap()
            >= 10
    );
}

#[tokio::test]
async fn user_crud_roundtrip() {
    let mut app = TestApp::new();
    app.login_admin().await;

    let created = app
        .post_json(
            "/api/v1/users",
            json!({ "name": "QA Bot", "username": "qa.bot", "email": "qa@filers.test",
                    "password": "secret12345", "role_ids": [2], "status": "active" }),
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED);
    let id = created.json["id"].as_str().unwrap().to_string();
    assert_eq!(created.json["role"], "user");

    // dup email → 409
    assert_eq!(
        app.post_json(
            "/api/v1/users",
            json!({ "name": "x", "email": "qa@filers.test", "password": "secret12345" })
        )
        .await
        .status,
        StatusCode::CONFLICT
    );

    // update → suspend + promote to admin
    let updated = app
        .put_json(
            &format!("/api/v1/users/{id}"),
            json!({ "status": "suspended", "role_ids": [1] }),
        )
        .await;
    assert_eq!(updated.status, StatusCode::OK);
    assert_eq!(updated.json["status"], "suspended");
    assert_eq!(updated.json["role"], "admin");

    assert_eq!(
        app.delete(&format!("/api/v1/users/{id}")).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        app.get(&format!("/api/v1/users/{id}")).await.status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn admin_cannot_delete_self() {
    let mut app = TestApp::new();
    let me = app.login_admin().await;
    let my_id = me.json["user"]["id"].as_str().unwrap().to_string();
    assert_eq!(
        app.delete(&format!("/api/v1/users/{my_id}")).await.status,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn role_crud_and_admin_protection() {
    let mut app = TestApp::new();
    app.login_admin().await;

    // create with permissions given by name (like the create form)
    let role = app
        .post_json(
            "/api/v1/roles",
            json!({ "name": "auditor", "permissions": ["users.view", "jobs.view"] }),
        )
        .await;
    assert_eq!(role.status, StatusCode::CREATED);
    let rid = role.json["id"].as_u64().unwrap();
    assert_eq!(role.json["permissions_count"], 2);

    // update permissions by id
    let updated = app
        .put_json(
            &format!("/api/v1/roles/{rid}"),
            json!({ "permissions": [1] }),
        )
        .await;
    assert_eq!(updated.json["permissions_count"], 1);

    assert_eq!(
        app.delete(&format!("/api/v1/roles/{rid}")).await.status,
        StatusCode::NO_CONTENT
    );
    // the admin role (id 1) is protected
    assert_eq!(
        app.delete("/api/v1/roles/1").await.status,
        StatusCode::CONFLICT
    );
}

#[tokio::test]
async fn permission_name_is_validated() {
    let mut app = TestApp::new();
    app.login_admin().await;

    assert_eq!(
        app.post_json("/api/v1/permissions", json!({ "name": "Bad Name!" }))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        app.post_json("/api/v1/permissions", json!({ "name": "users.view" }))
            .await
            .status,
        StatusCode::CONFLICT
    );
    assert_eq!(
        app.post_json("/api/v1/permissions", json!({ "name": "reports.export" }))
            .await
            .status,
        StatusCode::CREATED
    );
}
