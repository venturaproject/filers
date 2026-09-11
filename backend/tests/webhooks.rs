#![allow(clippy::unwrap_used)]
mod common;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::http::StatusCode;
use common::TestApp;
use serde_json::{Value, json};

/// A batch job POSTs its final state to `webhook_url` on completion.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn batch_completion_fires_the_webhook() {
    // A throwaway receiver that records the last body it got.
    let received: Arc<Mutex<Option<Value>>> = Arc::new(Mutex::new(None));
    let sink = received.clone();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let hook = axum::Router::new().route(
        "/hook",
        axum::routing::post(move |axum::Json(body): axum::Json<Value>| {
            let sink = sink.clone();
            async move {
                *sink.lock().unwrap() = Some(body);
                StatusCode::OK
            }
        }),
    );
    tokio::spawn(async move {
        axum::serve(listener, hook).await.unwrap();
    });

    let mut app = TestApp::new();
    let started = app
        .post_json_key(
            "/api/process/batch",
            "test-key",
            json!({ "webhook_url": format!("http://{addr}/hook") }),
        )
        .await;
    assert_eq!(started.status, StatusCode::OK);
    let job_id = started.json["job_id"].as_str().unwrap().to_string();

    // Wait for the callback (empty base dir → the job finishes fast).
    let mut got = None;
    for _ in 0..50 {
        if let Some(v) = received.lock().unwrap().clone() {
            got = Some(v);
            break;
        }
        tokio::time::sleep(Duration::from_millis(40)).await;
    }

    let payload = got.expect("webhook was delivered");
    assert_eq!(payload["event"], "job.completed");
    assert_eq!(payload["job"]["id"], job_id);
    assert_eq!(payload["job"]["status"], "completed");
}
