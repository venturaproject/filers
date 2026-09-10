//! `POST /api/pdf/text` — per-page text extraction.

use serde::Serialize;

use crate::errors::{AppError, AppResult};

/// Upper bound on pages processed in one call.
const MAX_PAGES: usize = 2_000;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct TextResult {
    pub pages: Vec<PageText>,
    /// Set when the request asked for more than `MAX_PAGES`.
    pub truncated: bool,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PageText {
    pub page: u32,
    pub text: String,
    /// Characters extracted — a quick "did we get anything" signal for the caller.
    pub chars: usize,
}

pub fn run(bytes: &[u8], pages: Option<&str>) -> AppResult<TextResult> {
    let doc = super::load(bytes)?;
    let total = doc.get_pages().len() as u32;
    if total == 0 {
        return Err(AppError::BadRequest("the PDF has no pages".into()));
    }

    let mut wanted = super::parse_pages(pages, total)?;
    let truncated = wanted.len() > MAX_PAGES;
    wanted.truncate(MAX_PAGES);

    let out = wanted
        .into_iter()
        .map(|page| {
            let text = doc.extract_text(&[page]).unwrap_or_default();
            let text = normalise(&text);
            PageText {
                page,
                chars: text.chars().count(),
                text,
            }
        })
        .collect();

    Ok(TextResult {
        pages: out,
        truncated,
    })
}

/// lopdf joins runs with no separator and keeps form-feed / CR noise — tidy it
/// into something a consumer can read without over-processing.
fn normalise(s: &str) -> String {
    s.replace(['\r', '\u{000c}'], "\n")
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}
