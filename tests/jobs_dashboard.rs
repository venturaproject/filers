#![allow(clippy::unwrap_used)]
mod common;

use axum::http::StatusCode;
use common::{TestApp, csv_bytes, png_bytes, xlsx_bytes};

const OCTET: &str = "application/octet-stream";

async fn wait_completed(app: &mut TestApp, job_id: &str) -> serde_json::Value {
    for _ in 0..50 {
        let r = app.get(&format!("/api/v1/jobs/{job_id}")).await;
        let status = r.json["status"].as_str().unwrap_or("");
        if status == "completed" || status == "failed" {
            return r.json;
        }
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;
    }
    panic!("job {job_id} never finished");
}

#[tokio::test]
async fn jobs_list_requires_admin() {
    let mut app = TestApp::new();
    assert_eq!(
        app.get("/api/v1/jobs").await.status,
        StatusCode::UNAUTHORIZED
    );
    app.login_user().await;
    assert_eq!(app.get("/api/v1/jobs").await.status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn admin_upload_creates_a_completed_batch_job() {
    let mut app = TestApp::new();
    app.login_admin().await;

    let created = app
        .post_files(
            "/api/v1/jobs",
            &[
                ("file", "clientes.csv", "text/csv", &csv_bytes()),
                ("file", "ventas.xlsx", OCTET, &xlsx_bytes()),
            ],
        )
        .await;
    assert_eq!(created.status, StatusCode::CREATED);
    let job_id = created.json["job_id"].as_str().unwrap().to_string();

    let detail = wait_completed(&mut app, &job_id).await;
    assert_eq!(detail["status"], "completed");
    assert_eq!(detail["kind"], "batch");
    assert_eq!(detail["origin"], "admin");
    assert_eq!(detail["files_total"], 2);
    assert_eq!(detail["files_processed"], 2);
    assert_eq!(detail["results"].as_array().unwrap().len(), 2);

    // shows up in the list + stats
    let list = app.get("/api/v1/jobs").await;
    assert_eq!(list.json["total"], 1);
    assert_eq!(list.json["stats"]["completed"], 1);

    // filter
    assert_eq!(app.get("/api/v1/jobs?origin=admin").await.json["total"], 1);
    assert_eq!(
        app.get("/api/v1/jobs?origin=api_key").await.json["total"],
        0
    );
    assert_eq!(app.get("/api/v1/jobs?status=failed").await.json["total"], 0);
}

#[tokio::test]
async fn empty_upload_is_rejected() {
    let mut app = TestApp::new();
    app.login_admin().await;
    let r = app
        .post_files("/api/v1/jobs", &[("notfile", "x", "text/plain", b"x")])
        .await;
    assert_eq!(r.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn dashboard_aggregates_real_activity() {
    let mut app = TestApp::new();

    // 2 sync via api key + 1 admin batch
    app.post_file_key(
        "/api/process",
        "test-key",
        "file",
        "a.csv",
        OCTET,
        &csv_bytes(),
    )
    .await;
    app.post_file_key(
        "/api/process",
        "test-key",
        "file",
        "b.csv",
        OCTET,
        &csv_bytes(),
    )
    .await;

    app.login_admin().await;
    let batch = app
        .post_files(
            "/api/v1/jobs",
            &[("file", "c.csv", "text/csv", &csv_bytes())],
        )
        .await;
    wait_completed(&mut app, batch.json["job_id"].as_str().unwrap()).await;

    let d = app.get("/api/v1/dashboard").await;
    assert_eq!(d.status, StatusCode::OK);
    assert_eq!(d.json["processings"]["total"], 3);
    assert_eq!(d.json["processings"]["by_kind"]["sync"], 2);
    assert_eq!(d.json["processings"]["by_kind"]["batch"], 1);
    assert_eq!(d.json["processings"]["by_origin"]["api_key"], 2);
    assert_eq!(d.json["processings"]["by_origin"]["admin"], 1);
    assert_eq!(d.json["counts"]["users"], 3);
    assert_eq!(d.json["counts"]["roles"], 2);
    assert!(d.json["recent"].as_array().unwrap().len() == 3);
}

#[tokio::test]
async fn avatar_upload() {
    let mut app = TestApp::new();
    app.login_admin().await;

    let ok = app
        .post_file(
            "/api/v1/auth/me/avatar",
            "avatar",
            "me.png",
            "image/png",
            &png_bytes(),
        )
        .await;
    assert_eq!(ok.status, StatusCode::OK);
    let avatar = ok.json["user"]["avatar"].as_str().unwrap();
    assert!(avatar.starts_with("data:image/png;base64,"));

    // persisted
    assert_eq!(
        app.get("/api/v1/auth/me").await.json["user"]["avatar"],
        avatar
    );

    // non-image rejected
    let bad = app
        .post_file(
            "/api/v1/auth/me/avatar",
            "avatar",
            "x.txt",
            "text/plain",
            b"hello",
        )
        .await;
    assert_eq!(bad.status, StatusCode::UNPROCESSABLE_ENTITY);
}
