//! `POST /api/pdf/info` — page count, page sizes, metadata, PDF version,
//! encryption and whether the document carries an AcroForm.

use lopdf::{Dictionary, Document, Object};
use serde::Serialize;

use super::decode_pdf_string;
use crate::errors::AppResult;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PdfInfo {
    pub pages: u32,
    pub pdf_version: String,
    pub encrypted: bool,
    pub has_form: bool,
    pub metadata: Metadata,
    pub page_sizes: Vec<PageSize>,
}

#[derive(Debug, Default, Serialize, utoipa::ToSchema)]
pub struct Metadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct PageSize {
    pub page: u32,
    /// PDF points (1/72 inch).
    pub width: f64,
    pub height: f64,
}

pub fn run(bytes: &[u8]) -> AppResult<PdfInfo> {
    let doc = super::load(bytes)?;

    let page_map = doc.get_pages();
    let encrypted = doc.trailer.get(b"Encrypt").is_ok();
    let has_form = catalog(&doc)
        .and_then(|c| c.get(b"AcroForm").ok())
        .is_some();
    let metadata = read_metadata(&doc);

    let page_sizes = page_map
        .iter()
        .map(|(num, id)| {
            let (width, height) = doc
                .get_dictionary(*id)
                .ok()
                .and_then(|d| media_box(&doc, d))
                .unwrap_or((0.0, 0.0));
            PageSize {
                page: *num,
                width,
                height,
            }
        })
        .collect();

    Ok(PdfInfo {
        pages: page_map.len() as u32,
        pdf_version: doc.version.clone(),
        encrypted,
        has_form,
        metadata,
        page_sizes,
    })
}

fn catalog(doc: &Document) -> Option<&Dictionary> {
    let root = doc.trailer.get(b"Root").ok()?;
    doc.get_object(root.as_reference().ok()?)
        .ok()?
        .as_dict()
        .ok()
}

fn read_metadata(doc: &Document) -> Metadata {
    let Some(info) = doc
        .trailer
        .get(b"Info")
        .ok()
        .and_then(|o| o.as_reference().ok())
        .and_then(|r| doc.get_object(r).ok())
        .and_then(|o| o.as_dict().ok())
    else {
        return Metadata::default();
    };

    let field = |key: &[u8]| {
        info.get(key)
            .ok()
            .and_then(|o| o.as_str().ok())
            .map(decode_pdf_string)
            .filter(|s| !s.is_empty())
    };

    Metadata {
        title: field(b"Title"),
        author: field(b"Author"),
        subject: field(b"Subject"),
        keywords: field(b"Keywords"),
        creator: field(b"Creator"),
        producer: field(b"Producer"),
    }
}

/// `/MediaBox` on the page, or inherited from an ancestor in the page tree.
fn media_box(doc: &Document, page: &Dictionary) -> Option<(f64, f64)> {
    let mut node = page;
    let mut guard = 0;
    loop {
        if let Ok(mb) = node.get(b"MediaBox").and_then(Object::as_array) {
            let v: Vec<f64> = mb.iter().filter_map(as_f64).collect();
            if v.len() == 4 {
                return Some(((v[2] - v[0]).abs(), (v[3] - v[1]).abs()));
            }
        }
        guard += 1;
        if guard > 32 {
            return None;
        }
        let parent = node.get(b"Parent").ok()?.as_reference().ok()?;
        node = doc.get_object(parent).ok()?.as_dict().ok()?;
    }
}

fn as_f64(o: &Object) -> Option<f64> {
    match o {
        Object::Integer(i) => Some(*i as f64),
        Object::Real(r) => Some(*r as f64),
        _ => None,
    }
}
