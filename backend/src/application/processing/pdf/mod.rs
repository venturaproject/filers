//! PDF processing — `/api/pdf/*`.
//!
//! Pure-Rust (lopdf), so there is no native dependency to bundle. Text
//! extraction is best-effort: it reads the text-showing operators and does
//! **not** OCR — a scanned PDF yields little or nothing.

pub mod forms;
pub mod info;
pub mod pages;
pub mod text;

use std::collections::BTreeSet;

use lopdf::Document;

use crate::errors::{AppError, AppResult};

/// Parse the bytes as a PDF, with a friendly error.
pub(crate) fn load(bytes: &[u8]) -> AppResult<Document> {
    if !bytes.starts_with(b"%PDF-") {
        return Err(AppError::BadRequest(
            "not a PDF (missing %PDF- header)".into(),
        ));
    }
    Document::load_mem(bytes).map_err(|e| AppError::ParseError(format!("cannot read PDF: {e}")))
}

/// A `1-3,7,10-12` selector → sorted, de-duplicated, 1-based page numbers
/// clamped to `[1, total]`. `None` / empty → every page.
pub(crate) fn parse_pages(spec: Option<&str>, total: u32) -> AppResult<Vec<u32>> {
    let Some(spec) = spec.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok((1..=total).collect());
    };

    let mut out: BTreeSet<u32> = BTreeSet::new();
    for part in spec.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        match part.split_once('-') {
            Some((a, b)) => {
                let a: u32 = a.trim().parse().map_err(|_| bad(part))?;
                let b: u32 = b.trim().parse().map_err(|_| bad(part))?;
                out.extend((a.min(b)..=a.max(b)).filter(|p| (1..=total).contains(p)));
            }
            None => {
                let p: u32 = part.parse().map_err(|_| bad(part))?;
                if (1..=total).contains(&p) {
                    out.insert(p);
                }
            }
        }
    }

    if out.is_empty() {
        return Err(AppError::BadRequest(
            "the page selector matched no pages".into(),
        ));
    }
    Ok(out.into_iter().collect())
}

fn bad(part: &str) -> AppError {
    AppError::BadRequest(format!("invalid page-selector segment: '{part}'"))
}

/// Decode a PDF text string (`Object::String`) — UTF-16BE when it carries a BOM,
/// otherwise treated as Latin-1 / PDFDocEncoding (close enough for metadata).
pub(crate) fn decode_pdf_string(raw: &[u8]) -> String {
    if raw.starts_with(&[0xFE, 0xFF]) {
        let u16s: Vec<u16> = raw[2..]
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&u16s)
    } else {
        raw.iter().map(|&b| b as char).collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use lopdf::content::{Content, Operation};
    use lopdf::{Object, Stream, dictionary};

    /// A minimal multi-page PDF with `Page N content` text on each page.
    pub(crate) fn sample_pdf(pages: usize) -> Vec<u8> {
        let mut doc = Document::with_version("1.7");
        let font_id = doc.add_object(dictionary! {
            "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
        });
        let pages_id = doc.new_object_id();

        let kids: Vec<Object> = (1..=pages)
            .map(|i| {
                let content = Content {
                    operations: vec![
                        Operation::new("BT", vec![]),
                        Operation::new("Tf", vec!["F1".into(), 24.into()]),
                        Operation::new("Td", vec![72.into(), 700.into()]),
                        Operation::new(
                            "Tj",
                            vec![Object::string_literal(format!("Page {i} content"))],
                        ),
                        Operation::new("ET", vec![]),
                    ],
                };
                let content_id =
                    doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
                let page_id = doc.add_object(dictionary! {
                    "Type" => "Page",
                    "Parent" => pages_id,
                    "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
                    "Contents" => content_id,
                    "Resources" => dictionary! { "Font" => dictionary! { "F1" => font_id } },
                });
                page_id.into()
            })
            .collect();

        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages", "Count" => pages as i64, "Kids" => kids,
            }),
        );
        let catalog_id = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog_id);

        let mut buf = std::io::Cursor::new(Vec::new());
        doc.save_to(&mut buf).unwrap();
        buf.into_inner()
    }

    /// `cargo test write_sample_fixture -- --ignored --nocapture` → sample PDFs
    /// under `/tmp` for poking at the live endpoints.
    #[test]
    #[ignore]
    fn write_sample_fixture() {
        std::fs::write("/tmp/sample-a.pdf", sample_pdf(3)).unwrap();
        std::fs::write("/tmp/sample-b.pdf", sample_pdf(2)).unwrap();
        eprintln!("wrote /tmp/sample-a.pdf (3 pages) and /tmp/sample-b.pdf (2 pages)");
    }

    #[test]
    fn rejects_non_pdf() {
        assert!(load(b"not a pdf at all").is_err());
    }

    #[test]
    fn page_selector() {
        assert_eq!(parse_pages(None, 5).unwrap(), vec![1, 2, 3, 4, 5]);
        assert_eq!(parse_pages(Some("1-3,7,2"), 10).unwrap(), vec![1, 2, 3, 7]);
        assert_eq!(parse_pages(Some("3-1"), 10).unwrap(), vec![1, 2, 3]);
        assert_eq!(parse_pages(Some("8-99"), 10).unwrap(), vec![8, 9, 10]);
        assert!(parse_pages(Some("50"), 10).is_err());
        assert!(parse_pages(Some("x"), 10).is_err());
    }

    #[test]
    fn info_reports_pages_and_size() {
        let out = info::run(&sample_pdf(3)).unwrap();
        assert_eq!(out.pages, 3);
        assert!(!out.encrypted);
        assert!(!out.has_form);
        assert_eq!(out.page_sizes.len(), 3);
        assert_eq!(out.page_sizes[0].width, 612.0);
        assert_eq!(out.page_sizes[0].height, 792.0);
    }

    #[test]
    fn text_is_per_page() {
        let out = text::run(&sample_pdf(3), Some("1,3")).unwrap();
        assert_eq!(out.pages.len(), 2);
        assert_eq!(out.pages[0].page, 1);
        assert!(out.pages[0].text.contains("Page 1 content"));
        assert_eq!(out.pages[1].page, 3);
        assert!(out.pages[1].text.contains("Page 3 content"));
    }

    #[test]
    fn forms_absent_on_a_plain_pdf() {
        let out = forms::run(&sample_pdf(1)).unwrap();
        assert!(!out.has_form);
        assert!(out.fields.is_empty());
    }

    #[test]
    fn split_keeps_only_the_selected_pages() {
        let out = pages::split(&sample_pdf(5), Some("2-3")).unwrap();
        let reparsed = info::run(&out).unwrap();
        assert_eq!(reparsed.pages, 2);
    }

    #[test]
    fn split_each_yields_one_pdf_per_page() {
        let files = pages::split_each(&sample_pdf(4), Some("1-2"), "doc").unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].0, "doc-p1.pdf");
        assert_eq!(info::run(&files[0].1).unwrap().pages, 1);
    }

    #[test]
    fn merge_concatenates() {
        let a = sample_pdf(2);
        let b = sample_pdf(3);
        let out = pages::merge(vec![a, b]).unwrap();
        assert_eq!(info::run(&out).unwrap().pages, 5);
    }

    #[test]
    fn merge_needs_two() {
        assert!(pages::merge(vec![sample_pdf(1)]).is_err());
    }
}
