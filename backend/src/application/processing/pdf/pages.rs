//! `POST /api/pdf/split` and `POST /api/pdf/merge`.

use std::io::{Cursor, Write};

use lopdf::{Document, Object, ObjectId};

use crate::errors::{AppError, AppResult};

/// Keep only `pages` (a `1-3,7` selector), return the new PDF bytes.
pub fn split(bytes: &[u8], pages: Option<&str>) -> AppResult<Vec<u8>> {
    let mut doc = super::load(bytes)?;
    let total = doc.get_pages().len() as u32;
    if total == 0 {
        return Err(AppError::BadRequest("the PDF has no pages".into()));
    }

    let keep: std::collections::BTreeSet<u32> =
        super::parse_pages(pages, total)?.into_iter().collect();
    let drop: Vec<u32> = (1..=total).filter(|p| !keep.contains(p)).collect();

    if drop.len() as u32 == total {
        return Err(AppError::BadRequest("that would remove every page".into()));
    }

    doc.delete_pages(&drop);
    doc.prune_objects();
    save(&mut doc)
}

/// One single-page PDF per selected page, as `(filename, bytes)` pairs — the
/// handler zips them.
pub fn split_each(
    bytes: &[u8],
    pages: Option<&str>,
    stem: &str,
) -> AppResult<Vec<(String, Vec<u8>)>> {
    let doc = super::load(bytes)?;
    let total = doc.get_pages().len() as u32;
    let wanted = super::parse_pages(pages, total)?;

    wanted
        .into_iter()
        .map(|p| {
            let mut one = doc.clone();
            let drop: Vec<u32> = (1..=total).filter(|n| *n != p).collect();
            one.delete_pages(&drop);
            one.prune_objects();
            Ok((format!("{stem}-p{p}.pdf"), save(&mut one)?))
        })
        .collect()
}

/// Concatenate several PDFs into one.
pub fn merge(docs: Vec<Vec<u8>>) -> AppResult<Vec<u8>> {
    if docs.len() < 2 {
        return Err(AppError::BadRequest(
            "merge needs at least two `file` parts".into(),
        ));
    }

    let mut merged = Document::with_version("1.7");
    let mut max_id = 1u32;
    let mut page_ids: Vec<ObjectId> = Vec::new();

    for (i, bytes) in docs.into_iter().enumerate() {
        let mut doc = super::load(&bytes).map_err(|e| {
            AppError::BadRequest(format!("file {} is not a readable PDF: {e}", i + 1))
        })?;

        // Shift every object id in this document past the ones already merged.
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;

        let pages = doc.get_pages();
        for (_, id) in pages {
            page_ids.push(id);
        }
        merged.objects.extend(doc.objects);
    }

    // A fresh Pages tree pointing at every collected page, then a Catalog.
    let pages_id = merged.new_object_id();
    for id in &page_ids {
        if let Ok(dict) = merged.get_object_mut(*id).and_then(Object::as_dict_mut) {
            dict.set("Parent", pages_id);
        }
    }

    let mut pages_dict = lopdf::Dictionary::new();
    pages_dict.set("Type", "Pages");
    pages_dict.set("Count", page_ids.len() as i64);
    pages_dict.set(
        "Kids",
        page_ids
            .iter()
            .map(|id| Object::Reference(*id))
            .collect::<Vec<_>>(),
    );
    merged
        .objects
        .insert(pages_id, Object::Dictionary(pages_dict));

    let catalog_id = merged.new_object_id();
    let mut catalog = lopdf::Dictionary::new();
    catalog.set("Type", "Catalog");
    catalog.set("Pages", pages_id);
    merged
        .objects
        .insert(catalog_id, Object::Dictionary(catalog));

    merged.trailer.set("Root", catalog_id);
    merged.prune_objects();
    save(&mut merged)
}

fn save(doc: &mut Document) -> AppResult<Vec<u8>> {
    let mut buf = Cursor::new(Vec::new());
    doc.save_to(&mut buf)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("cannot write PDF: {e}")))?;
    buf.flush().ok();
    Ok(buf.into_inner())
}
