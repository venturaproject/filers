//! Shared harness for the integration tests: an in-memory app driven through
//! `tower::ServiceExt::oneshot` (no sockets), with a session cookie jar.

#![allow(dead_code, clippy::unwrap_used)]

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

pub const BOUNDARY: &str = "TESTBOUNDARY0xCAFE";

pub struct TestApp {
    router: Router,
    cookie: Option<String>,
    /// Kept alive so `batch_base_dir` exists for the whole test.
    _tmp: tempfile::TempDir,
}

pub struct Resp {
    pub status: StatusCode,
    pub json: Value,
    pub body: Vec<u8>,
    pub set_cookie: Option<String>,
    pub headers: axum::http::HeaderMap,
}

impl Resp {
    pub fn text(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.body)
    }
}

impl Resp {
    pub fn ok(&self) -> bool {
        self.status.is_success()
    }
}

impl TestApp {
    pub fn new() -> Self {
        let tmp = tempfile::tempdir().expect("tempdir");
        let config = rust_api::bootstrap::test_config(tmp.path().to_string_lossy().to_string());
        Self {
            router: rust_api::bootstrap::build_app(config),
            cookie: None,
            _tmp: tmp,
        }
    }

    pub fn batch_dir(&self) -> &std::path::Path {
        self._tmp.path()
    }

    pub async fn raw(&mut self, req: Request<Body>) -> Resp {
        self.send(req).await
    }

    async fn send(&mut self, req: Request<Body>) -> Resp {
        let response = self
            .router
            .clone()
            .oneshot(req)
            .await
            .expect("router response");

        let status = response.status();
        let headers = response.headers().clone();
        let set_cookie = headers
            .get(header::SET_COOKIE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);

        // Remember the session cookie for subsequent requests.
        if let Some(sc) = &set_cookie
            && let Some(pair) = sc.split(';').next()
            && let Some(val) = pair.strip_prefix("session=")
        {
            self.cookie = (!val.is_empty()).then(|| pair.to_string());
        }

        let bytes = response
            .into_body()
            .collect()
            .await
            .expect("collect body")
            .to_bytes();
        let json = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap_or(Value::Null)
        };

        Resp {
            status,
            json,
            body: bytes.to_vec(),
            set_cookie,
            headers,
        }
    }

    fn base(&self, method: &str, path: &str) -> axum::http::request::Builder {
        let mut b = Request::builder().method(method).uri(path);
        if let Some(c) = &self.cookie {
            b = b.header(header::COOKIE, c);
        }
        b
    }

    pub async fn get(&mut self, path: &str) -> Resp {
        let req = self.base("GET", path).body(Body::empty()).unwrap();
        self.send(req).await
    }

    /// GET with an `Authorization: Bearer` header.
    pub async fn get_bearer(&mut self, path: &str, token: &str) -> Resp {
        let req = self
            .base("GET", path)
            .header(header::AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .unwrap();
        self.send(req).await
    }

    /// GET with an `x-api-key` header.
    pub async fn get_key(&mut self, path: &str, key: &str) -> Resp {
        let req = self
            .base("GET", path)
            .header("x-api-key", key)
            .body(Body::empty())
            .unwrap();
        self.send(req).await
    }

    pub async fn delete(&mut self, path: &str) -> Resp {
        let req = self.base("DELETE", path).body(Body::empty()).unwrap();
        self.send(req).await
    }

    pub async fn post_empty(&mut self, path: &str) -> Resp {
        let req = self.base("POST", path).body(Body::empty()).unwrap();
        self.send(req).await
    }

    pub async fn json(&mut self, method: &str, path: &str, body: Value) -> Resp {
        self.json_with(method, path, &[], body).await
    }

    pub async fn json_with(
        &mut self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: Value,
    ) -> Resp {
        let mut b = self
            .base(method, path)
            .header(header::CONTENT_TYPE, "application/json");
        for (name, value) in headers {
            b = b.header(*name, *value);
        }
        self.send(b.body(Body::from(body.to_string())).unwrap())
            .await
    }

    pub async fn post_json(&mut self, path: &str, body: Value) -> Resp {
        self.json("POST", path, body).await
    }

    /// POST JSON with an `x-api-key` header (for the processing endpoints).
    pub async fn post_json_key(&mut self, path: &str, key: &str, body: Value) -> Resp {
        self.json_with("POST", path, &[("x-api-key", key)], body)
            .await
    }
    pub async fn put_json(&mut self, path: &str, body: Value) -> Resp {
        self.json("PUT", path, body).await
    }
    pub async fn patch_json(&mut self, path: &str, body: Value) -> Resp {
        self.json("PATCH", path, body).await
    }

    /// One-part `multipart/form-data` upload.
    pub async fn post_file(
        &mut self,
        path: &str,
        field: &str,
        filename: &str,
        content_type: &str,
        bytes: &[u8],
    ) -> Resp {
        self.post_files(path, &[(field, filename, content_type, bytes)])
            .await
    }

    pub async fn post_files(&mut self, path: &str, parts: &[(&str, &str, &str, &[u8])]) -> Resp {
        self.multipart(path, &[], parts).await
    }

    /// Multipart upload authenticated with a Bearer access token (no session).
    pub async fn post_file_bearer(
        &mut self,
        path: &str,
        bearer: &str,
        field: &str,
        filename: &str,
        content_type: &str,
        bytes: &[u8],
    ) -> Resp {
        self.multipart(
            path,
            &[("authorization", format!("Bearer {bearer}"))],
            &[(field, filename, content_type, bytes)],
        )
        .await
    }

    /// Multipart upload with an `x-api-key` header.
    pub async fn post_file_key(
        &mut self,
        path: &str,
        key: &str,
        field: &str,
        filename: &str,
        content_type: &str,
        bytes: &[u8],
    ) -> Resp {
        self.multipart(
            path,
            &[("x-api-key", key.to_string())],
            &[(field, filename, content_type, bytes)],
        )
        .await
    }

    /// Multi-part multipart upload with an `x-api-key` header.
    pub async fn post_files_key(
        &mut self,
        path: &str,
        key: &str,
        parts: &[(&str, &str, &str, &[u8])],
    ) -> Resp {
        self.multipart(path, &[("x-api-key", key.to_string())], parts)
            .await
    }

    async fn multipart(
        &mut self,
        path: &str,
        headers: &[(&str, String)],
        parts: &[(&str, &str, &str, &[u8])],
    ) -> Resp {
        let mut body: Vec<u8> = Vec::new();
        for (field, filename, ct, bytes) in parts {
            body.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
            body.extend_from_slice(
                format!(
                    "Content-Disposition: form-data; name=\"{field}\"; filename=\"{filename}\"\r\n"
                )
                .as_bytes(),
            );
            body.extend_from_slice(format!("Content-Type: {ct}\r\n\r\n").as_bytes());
            body.extend_from_slice(bytes);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());

        let mut b = self.base("POST", path).header(
            header::CONTENT_TYPE,
            format!("multipart/form-data; boundary={BOUNDARY}"),
        );
        for (name, value) in headers {
            b = b.header(*name, value);
        }
        self.send(b.body(Body::from(body)).unwrap()).await
    }

    // ── Auth shortcuts ─────────────────────────────────────────────────────

    pub async fn login(&mut self, email: &str, password: &str) -> Resp {
        self.post_json(
            "/api/v1/auth/login",
            json!({ "login": email, "password": password }),
        )
        .await
    }

    pub async fn login_admin(&mut self) -> Resp {
        self.login("admin@filers.test", "secret12345").await
    }

    pub async fn login_user(&mut self) -> Resp {
        self.login("maria@filers.test", "demo1234").await
    }

    pub fn logout_local(&mut self) {
        self.cookie = None;
    }

    /// Grab / restore the active session cookie, to juggle two identities in one
    /// test (e.g. a user session that an admin then invalidates).
    pub fn snapshot_cookie(&self) -> Option<String> {
        self.cookie.clone()
    }

    pub fn restore_cookie(&mut self, cookie: Option<String>) {
        self.cookie = cookie;
    }
}

// ── Fixtures ───────────────────────────────────────────────────────────────

pub fn csv_bytes() -> Vec<u8> {
    b"name;age;city\nAna;34;Madrid\nLuis;28;Sevilla\n".to_vec()
}

/// The committed sample spreadsheet (`tests/fixtures/ventas.xlsx`).
pub fn xlsx_bytes() -> Vec<u8> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/ventas.xlsx");
    std::fs::read(path).expect("read tests/fixtures/ventas.xlsx")
}

/// A 1x1 red PNG (raw bytes — the magic-byte sniff only needs `\x89PNG`).
pub fn png_bytes() -> Vec<u8> {
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xCF, 0xC0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB0, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ]
}
