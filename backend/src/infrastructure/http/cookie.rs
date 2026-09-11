//! Minimal cookie helpers — one HttpOnly session cookie, no external crate.

use axum::http::{HeaderMap, HeaderValue, header::COOKIE};

pub const SESSION_COOKIE: &str = "session";
const MAX_AGE_SECONDS: i64 = 60 * 60 * 24 * 7;

/// Read a cookie value from the request `Cookie` header.
pub fn read(headers: &HeaderMap, name: &str) -> Option<String> {
    let raw = headers.get(COOKIE)?.to_str().ok()?;
    raw.split(';').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k.trim() == name).then(|| v.trim().to_string())
    })
}

/// `Set-Cookie` value that installs the session cookie.
pub fn set(token: &str, secure: bool) -> HeaderValue {
    build(token, MAX_AGE_SECONDS, secure)
}

/// `Set-Cookie` value that clears the session cookie.
pub fn clear(secure: bool) -> HeaderValue {
    build("", 0, secure)
}

fn build(value: &str, max_age: i64, secure: bool) -> HeaderValue {
    let mut s =
        format!("{SESSION_COOKIE}={value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age}");
    if secure {
        s.push_str("; Secure");
    }
    // Values here are ASCII (alphanumeric token or empty), so this never fails.
    HeaderValue::from_str(&s).expect("valid cookie header")
}
