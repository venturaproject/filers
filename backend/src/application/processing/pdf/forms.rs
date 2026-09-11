//! `POST /api/pdf/forms` — AcroForm field names, types and current values.

use lopdf::{Dictionary, Document, Object};
use serde::Serialize;

use super::decode_pdf_string;
use crate::errors::AppResult;

const MAX_FIELDS: usize = 10_000;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct FormResult {
    pub has_form: bool,
    pub fields: Vec<Field>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct Field {
    /// Fully-qualified name (`parent.child` for hierarchical fields).
    pub name: String,
    /// `text` | `button` | `choice` | `signature` | `unknown`.
    pub kind: String,
    pub value: Option<String>,
    /// The `/TU` tooltip / accessible label, when present.
    pub label: Option<String>,
}

pub fn run(bytes: &[u8]) -> AppResult<FormResult> {
    let doc = super::load(bytes)?;

    let Some(acroform) =
        catalog(&doc).and_then(|c| c.get(b"AcroForm").ok().and_then(|o| resolve_dict(&doc, o)))
    else {
        return Ok(FormResult {
            has_form: false,
            fields: Vec::new(),
        });
    };

    let mut fields = Vec::new();
    if let Ok(roots) = acroform.get(b"Fields").and_then(Object::as_array) {
        for f in roots {
            walk(&doc, f, None, &mut fields);
            if fields.len() >= MAX_FIELDS {
                break;
            }
        }
    }
    fields.truncate(MAX_FIELDS);

    Ok(FormResult {
        has_form: true,
        fields,
    })
}

fn walk(doc: &Document, obj: &Object, parent: Option<&str>, out: &mut Vec<Field>) {
    let Some(dict) = resolve_dict(doc, obj) else {
        return;
    };

    let partial = dict
        .get(b"T")
        .ok()
        .and_then(|o| o.as_str().ok())
        .map(decode_pdf_string);

    let name = match (parent, partial.as_deref()) {
        (Some(p), Some(t)) => format!("{p}.{t}"),
        (None, Some(t)) => t.to_string(),
        (Some(p), None) => p.to_string(),
        (None, None) => String::new(),
    };

    if let Ok(kids) = dict.get(b"Kids").and_then(Object::as_array) {
        // A node with kids that also has /FT is a terminal field with widget
        // kids; treat it as a leaf. Otherwise recurse.
        if dict.get(b"FT").is_err() {
            for kid in kids {
                walk(doc, kid, Some(&name), out);
                if out.len() >= MAX_FIELDS {
                    return;
                }
            }
            return;
        }
    }

    let kind = match dict.get(b"FT").ok().and_then(|o| o.as_name().ok()) {
        Some(b"Tx") => "text",
        Some(b"Btn") => "button",
        Some(b"Ch") => "choice",
        Some(b"Sig") => "signature",
        _ => "unknown",
    };

    out.push(Field {
        name,
        kind: kind.to_string(),
        value: field_value(doc, dict),
        label: dict
            .get(b"TU")
            .ok()
            .and_then(|o| o.as_str().ok())
            .map(decode_pdf_string)
            .filter(|s| !s.is_empty()),
    });
}

fn field_value(doc: &Document, dict: &Dictionary) -> Option<String> {
    let v = dict.get(b"V").ok()?;
    match v {
        Object::Name(n) => Some(String::from_utf8_lossy(n).into_owned()),
        Object::String(s, _) => Some(decode_pdf_string(s)),
        Object::Boolean(b) => Some(b.to_string()),
        Object::Integer(i) => Some(i.to_string()),
        Object::Real(r) => Some(r.to_string()),
        Object::Reference(id) => doc
            .get_object(*id)
            .ok()
            .and_then(|o| o.as_str().ok())
            .map(decode_pdf_string),
        _ => None,
    }
}

fn catalog(doc: &Document) -> Option<&Dictionary> {
    let root = doc.trailer.get(b"Root").ok()?;
    resolve_dict(doc, root)
}

fn resolve_dict<'a>(doc: &'a Document, obj: &'a Object) -> Option<&'a Dictionary> {
    match obj {
        Object::Dictionary(d) => Some(d),
        Object::Reference(id) => doc.get_object(*id).ok()?.as_dict().ok(),
        _ => None,
    }
}
