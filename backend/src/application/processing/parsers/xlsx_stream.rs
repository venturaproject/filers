//! Streaming xlsx reader.
//!
//! Counts rows and reads the header row by scanning `<row>`/`<c>` events with
//! quick-xml instead of building calamine's dense in-memory `Range`. Used for
//! `count_only` parses (batch jobs, stats) where a large file would otherwise
//! be fully materialised. Anything unexpected returns `Err` and the caller
//! ([`super::excel::parse_with_limits`]) falls back to the reference parser.

use std::borrow::Cow;
use std::io::{Cursor, Read};
use std::time::Instant;

use quick_xml::Reader;
use quick_xml::XmlVersion;
use quick_xml::events::{BytesStart, Event};

use crate::application::processing::operations::convert::col_letter;
use crate::application::processing::timing::process_cpu_time;
use crate::domain::processing::entities::{
    FileFormat, ParseOptions, ParseStats, ParsedFile, Timings,
};
use crate::errors::{AppError, AppResult};

type Zip = zip::ZipArchive<Cursor<Vec<u8>>>;

/// Count rows + read the header of `opts.sheet` without materialising the sheet.
pub fn count(bytes: &[u8], opts: &ParseOptions) -> AppResult<ParsedFile> {
    let start = Instant::now();
    let cpu_start = process_cpu_time();

    let mut zip = zip::ZipArchive::new(Cursor::new(bytes.to_vec()))
        .map_err(|e| AppError::ParseError(format!("xlsx zip: {e}")))?;

    let sheet_path = resolve_sheet_path(&mut zip, opts.sheet).map_err(AppError::ParseError)?;
    let shared = read_shared_strings(&mut zip).map_err(AppError::ParseError)?;
    let scan = scan_sheet(&mut zip, &sheet_path, &shared, opts).map_err(AppError::ParseError)?;

    let parse_ms = start.elapsed().as_millis();
    let timings = Timings {
        read_ms: parse_ms,
        parse_ms,
        parse_cpu_ms: process_cpu_time().saturating_sub(cpu_start).as_millis(),
        ..Default::default()
    };

    Ok(ParsedFile {
        format: FileFormat::Xlsx,
        stats: ParseStats {
            total_rows: scan.body_rows,
            returned_rows: 0,
            columns: scan.columns.len() as u32,
            elapsed_ms: parse_ms,
        },
        columns: scan.columns,
        data: Vec::new(),
        errors: Vec::new(),
        timings,
    })
}

// ── helpers ─────────────────────────────────────────────────────────────────

/// One attribute's value, owned.
fn attr(e: &BytesStart, key: &str) -> Option<String> {
    raw_attr(e, key).map(Cow::into_owned)
}

/// One attribute's value, borrowed where possible (hot path).
fn raw_attr<'a>(e: &'a BytesStart, key: &str) -> Option<Cow<'a, str>> {
    e.try_get_attribute(key).ok().flatten().map(|a| a.value)
}

fn read_zip_string(zip: &mut Zip, name: &str) -> Result<String, String> {
    let mut f = zip.by_name(name).map_err(|e| format!("{name}: {e}"))?;
    let mut s = String::new();
    f.read_to_string(&mut s)
        .map_err(|e| format!("{name}: {e}"))?;
    Ok(s)
}

/// Column index from a cell ref like `AB12` → 27 (0-based).
fn col_from_ref(r: &str) -> Option<usize> {
    let mut n: usize = 0;
    let mut any = false;
    for b in r.bytes() {
        if b.is_ascii_alphabetic() {
            any = true;
            n = n * 26 + (b.to_ascii_uppercase() - b'A' + 1) as usize;
        } else {
            break;
        }
    }
    any.then(|| n - 1)
}

// ── sheet resolution ────────────────────────────────────────────────────────

/// `xl/workbook.xml` lists sheets in order with an `r:id`; the rels file maps
/// that id to a path. Falls back to the conventional `worksheets/sheetN.xml`.
fn resolve_sheet_path(zip: &mut Zip, index: usize) -> Result<String, String> {
    let wb = read_zip_string(zip, "xl/workbook.xml")?;
    let mut r = Reader::from_str(&wb);
    let mut rids: Vec<String> = Vec::new();
    loop {
        match r.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().0 == "sheet" => {
                if let Some(id) = attr(&e, "r:id").or_else(|| attr(&e, "id")) {
                    rids.push(id);
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("workbook.xml: {e}")),
            _ => {}
        }
    }

    let rid = rids
        .get(index)
        .ok_or_else(|| format!("Sheet {index} not found"))?
        .clone();

    let rels = read_zip_string(zip, "xl/_rels/workbook.xml.rels")?;
    let mut rr = Reader::from_str(&rels);
    loop {
        match rr.read_event() {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) if e.name().0 == "Relationship" => {
                if attr(&e, "Id").as_deref() == Some(rid.as_str()) {
                    let target = attr(&e, "Target").ok_or("relationship has no Target")?;
                    return Ok(normalise_target(&target));
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("workbook.xml.rels: {e}")),
            _ => {}
        }
    }

    Ok(format!("xl/worksheets/sheet{}.xml", index + 1))
}

fn normalise_target(t: &str) -> String {
    let t = t.trim_start_matches('/');
    if t.starts_with("xl/") {
        t.to_string()
    } else {
        format!("xl/{t}")
    }
}

// ── shared strings ──────────────────────────────────────────────────────────

fn read_shared_strings(zip: &mut Zip) -> Result<Vec<String>, String> {
    let xml = match read_zip_string(zip, "xl/sharedStrings.xml") {
        Ok(x) => x,
        Err(_) => return Ok(Vec::new()), // absent → every string is inline / numeric
    };

    let mut r = Reader::from_str(&xml);
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut depth_si = 0usize;
    let mut in_t = false;

    loop {
        match r.read_event() {
            Ok(Event::Start(e)) => match e.name().0 {
                "si" => {
                    depth_si += 1;
                    cur.clear();
                }
                "t" if depth_si > 0 => in_t = true,
                _ => {}
            },
            Ok(Event::Text(t)) if in_t => {
                cur.push_str(&t.xml_content(XmlVersion::Implicit1_0));
            }
            Ok(Event::End(e)) => match e.name().0 {
                "t" => in_t = false,
                "si" if depth_si > 0 => {
                    depth_si -= 1;
                    out.push(std::mem::take(&mut cur));
                }
                _ => {}
            },
            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("sharedStrings.xml: {e}")),
            _ => {}
        }
    }
    Ok(out)
}

// ── sheet scan ──────────────────────────────────────────────────────────────

struct Scan {
    columns: Vec<String>,
    body_rows: u64,
}

enum CellKind {
    Shared,
    Inline,
    Str,
    Other,
}

fn cell_kind(e: &BytesStart) -> CellKind {
    match attr(e, "t").as_deref() {
        Some("s") => CellKind::Shared,
        Some("inlineStr") => CellKind::Inline,
        Some("str") => CellKind::Str,
        _ => CellKind::Other,
    }
}

/// `dimension ref="A1:NK3104"` → column count (NK → 375).
fn dim_width(e: &BytesStart) -> Option<usize> {
    let r = attr(e, "ref")?;
    let last = r.split(':').next_back().unwrap_or(r.as_str());
    col_from_ref(last).map(|c| c + 1)
}

fn resolve_text(kind: &CellKind, raw: &str, shared: &[String]) -> String {
    match kind {
        CellKind::Shared => raw
            .trim()
            .parse::<usize>()
            .ok()
            .and_then(|i| shared.get(i))
            .cloned()
            .unwrap_or_default(),
        CellKind::Inline | CellKind::Str => raw.to_string(),
        // A numeric header stringifies (matches calamine); anything else → "".
        CellKind::Other => {
            let t = raw.trim();
            if t.parse::<f64>().is_ok() {
                t.to_string()
            } else {
                String::new()
            }
        }
    }
}

fn scan_sheet(
    zip: &mut Zip,
    path: &str,
    shared: &[String],
    opts: &ParseOptions,
) -> Result<Scan, String> {
    let xml = read_zip_string(zip, path)?;
    let mut r = Reader::from_str(&xml);

    let header_ord = opts.skip_rows; // 0-based ordinal of the header row
    let mut in_sheet_data = false;
    let mut row_ord: usize = 0;
    let mut body_rows: u64 = 0;
    let mut declared_width: Option<usize> = None;
    let mut seen_width: usize = 0;

    // header row capture
    let mut capturing = false;
    let mut header_cells: Vec<(usize, String)> = Vec::new();

    // current cell
    let mut cell_col: usize = 0;
    let mut next_col: usize = 0; // implicit column for a `<c>` with no `r=`
    let mut kind = CellKind::Other;
    let mut cell_val = String::new();
    let mut in_v = false;
    let mut in_inline_t = false;

    loop {
        match r.read_event() {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) if e.name().0 == "dimension" => {
                declared_width = declared_width.or_else(|| dim_width(&e));
            }
            Ok(Event::Start(e)) if e.name().0 == "sheetData" => in_sheet_data = true,
            Ok(Event::End(e)) if e.name().0 == "sheetData" => break,

            Ok(Event::Start(e)) if in_sheet_data && e.name().0 == "row" => {
                capturing = opts.has_headers && row_ord == header_ord;
                header_cells.clear();
                next_col = 0;
            }
            Ok(Event::End(e)) if in_sheet_data && e.name().0 == "row" => {
                let is_body = if opts.has_headers {
                    row_ord > header_ord
                } else {
                    row_ord >= opts.skip_rows
                };
                if is_body {
                    body_rows += 1;
                }
                row_ord += 1;
                capturing = false;
            }

            Ok(Event::Empty(e)) if in_sheet_data && e.name().0 == "c" => {
                let col = raw_attr(&e, "r")
                    .and_then(|v| col_from_ref(&v))
                    .unwrap_or(next_col);
                next_col = col + 1;
                seen_width = seen_width.max(col + 1);
            }
            Ok(Event::Start(e)) if in_sheet_data && e.name().0 == "c" => {
                cell_col = raw_attr(&e, "r")
                    .and_then(|v| col_from_ref(&v))
                    .unwrap_or(next_col);
                next_col = cell_col + 1;
                seen_width = seen_width.max(cell_col + 1);
                if capturing {
                    kind = cell_kind(&e);
                    cell_val.clear();
                }
            }
            Ok(Event::End(e)) if capturing && in_sheet_data && e.name().0 == "c" => {
                header_cells.push((cell_col, resolve_text(&kind, &cell_val, shared)));
            }

            Ok(Event::Start(e)) if capturing && e.name().0 == "v" => in_v = true,
            Ok(Event::End(e)) if e.name().0 == "v" => in_v = false,
            Ok(Event::Start(e)) if capturing && e.name().0 == "t" => in_inline_t = true,
            Ok(Event::End(e)) if e.name().0 == "t" => in_inline_t = false,
            Ok(Event::Text(t)) if in_v || in_inline_t => {
                cell_val.push_str(&t.xml_content(XmlVersion::Implicit1_0));
            }

            Ok(Event::Eof) => break,
            Err(e) => return Err(format!("{path}: {e}")),
            _ => {}
        }
    }

    // Prefer the actual data extent (matches calamine, which trims to it); the
    // `<dimension>` tag is only a fallback for a sheet we somehow saw no cells in.
    let header_width = header_cells.iter().map(|(c, _)| c + 1).max().unwrap_or(0);
    let width = if seen_width == 0 && header_width == 0 {
        declared_width.unwrap_or(0)
    } else {
        seen_width.max(header_width)
    };

    let columns = if opts.has_headers {
        let mut h = vec![String::new(); width];
        for (c, text) in header_cells {
            if c < h.len() {
                h[c] = text.trim().to_string();
            }
        }
        h
    } else {
        (0..width).map(col_letter).collect()
    };

    Ok(Scan { columns, body_rows })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::application::processing::parsers::excel;
    use crate::domain::processing::entities::FileFormat;

    fn fixture() -> Vec<u8> {
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/ventas.xlsx"
        ))
        .expect("fixture")
    }

    #[test]
    fn stream_count_matches_calamine() {
        let bytes = fixture();
        let opts = ParseOptions {
            count_only: true,
            ..ParseOptions::default()
        };

        let streamed = count(&bytes, &opts).unwrap();
        let dense = excel::parse(&bytes, FileFormat::Xlsx, &opts).unwrap();

        assert_eq!(streamed.stats.total_rows, dense.stats.total_rows);
        assert_eq!(streamed.stats.columns, dense.stats.columns);
        assert_eq!(streamed.columns, dense.columns);
    }

    #[test]
    fn stream_count_honours_no_headers_and_skip() {
        let bytes = fixture();
        let opts = ParseOptions {
            count_only: true,
            has_headers: false,
            skip_rows: 2,
            ..ParseOptions::default()
        };

        let streamed = count(&bytes, &opts).unwrap();
        let dense = excel::parse(&bytes, FileFormat::Xlsx, &opts).unwrap();
        assert_eq!(streamed.stats.total_rows, dense.stats.total_rows);
        assert_eq!(streamed.stats.columns, dense.stats.columns);
    }
}
