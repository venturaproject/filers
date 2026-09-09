#![allow(clippy::unwrap_used)]
//! Timing harness for the spreadsheet parsers.
//!   cargo test --release --test perf -- --ignored --nocapture

use std::time::Instant;

use calamine::{Data, DataType, Reader, open_workbook_auto_from_rs};

fn load() -> Option<Vec<u8>> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/input.xlsx");
    match std::fs::read(path) {
        Ok(b) => Some(b),
        Err(_) => {
            eprintln!("skip: {path} not present");
            None
        }
    }
}

#[test]
#[ignore]
fn phased_breakdown() {
    let Some(bytes) = load() else { return };
    eprintln!("file: {} bytes", bytes.len());

    for run in 1..=3 {
        let t = Instant::now();
        let mut wb = open_workbook_auto_from_rs(std::io::Cursor::new(&bytes[..])).unwrap();
        let t_open = t.elapsed();

        let name = wb.sheet_names()[0].clone();
        let t = Instant::now();
        let range = wb.worksheet_range(&name).unwrap();
        let t_range = t.elapsed();

        let (h, w) = range.get_size();

        // build Vec<Vec<serde_json::Value>>
        let t = Instant::now();
        let mut data: Vec<Vec<serde_json::Value>> = Vec::with_capacity(h);
        for row in range.rows() {
            let mut cells = Vec::with_capacity(w);
            for c in row {
                cells.push(match c {
                    Data::Empty => serde_json::Value::Null,
                    Data::String(s) => serde_json::Value::String(s.clone()),
                    Data::Float(f) => serde_json::json!(f),
                    Data::Int(i) => serde_json::json!(i),
                    Data::Bool(b) => serde_json::json!(b),
                    _ => c
                        .as_datetime()
                        .map(|d| serde_json::Value::String(d.to_string()))
                        .unwrap_or(serde_json::Value::Null),
                });
            }
            data.push(cells);
        }
        let t_values = t.elapsed();

        // serialize to JSON bytes
        let t = Instant::now();
        let json = serde_json::to_vec(&data).unwrap();
        let t_json = t.elapsed();

        let t = Instant::now();
        drop(data);
        let t_drop = t.elapsed();

        eprintln!(
            "run {run}: {h}x{w}  open={t_open:?}  range={t_range:?}  values={t_values:?}  json={t_json:?} ({} MB)  drop={t_drop:?}",
            json.len() / 1_048_576
        );
    }
}

#[test]
#[ignore]
fn count_only_vs_full() {
    use rust_api::application::processing::parsers::excel::parse;
    use rust_api::domain::processing::entities::{FileFormat, ParseOptions};

    let Some(bytes) = load() else { return };

    let bench = |label: &str, opts: &ParseOptions| {
        let mut best = std::time::Duration::MAX;
        let mut rows = 0;
        for _ in 0..8 {
            let t = Instant::now();
            let out = parse(&bytes, FileFormat::Xlsx, opts).unwrap();
            best = best.min(t.elapsed());
            rows = out.stats.total_rows;
        }
        eprintln!("{label:<24} best-of-8 = {best:?}  ({rows} rows)");
    };

    let full = ParseOptions::default();
    let count = ParseOptions {
        count_only: true,
        ..ParseOptions::default()
    };
    let page = ParseOptions {
        max_rows: Some(100),
        ..ParseOptions::default()
    };

    bench("full (data + rayon)", &full);
    bench("count-only (batch)", &count);
    bench("paginated max_rows=100", &page);
}
