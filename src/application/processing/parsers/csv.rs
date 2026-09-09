use std::time::Instant;

use rayon::prelude::*;
use serde_json::Value;

use crate::domain::processing::entities::{
    FileFormat, ParseError, ParseOptions, ParseStats, ParsedFile,
};
use crate::errors::{AppError, AppResult};

pub fn parse(bytes: &[u8], opts: &ParseOptions) -> AppResult<ParsedFile> {
    let start = Instant::now();

    let delimiter = opts
        .delimiter
        .map(|c| c as u8)
        .unwrap_or_else(|| detect_delimiter(bytes));

    let mut builder = csv::ReaderBuilder::new();
    builder
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true);

    let mut reader = builder.from_reader(bytes);
    let mut records = reader.records();

    // skip leading rows
    for _ in 0..opts.skip_rows {
        records.next();
    }

    // headers
    let columns: Vec<String> = if opts.has_headers {
        match records.next() {
            Some(Ok(rec)) => rec.iter().map(|s| s.trim().to_string()).collect(),
            Some(Err(e)) => return Err(AppError::ParseError(format!("Header row error: {e}"))),
            None => return Ok(empty_result(opts, start)),
        }
    } else {
        vec![]
    };

    let base_row: u64 = opts.skip_rows as u64 + if opts.has_headers { 1 } else { 0 };

    // Stream the reader once: count every row, but only buffer the records that
    // fall inside the pagination window (kept for parallel cell typing below).
    let window_hi = opts.max_rows.map(|m| opts.offset.saturating_add(m));
    let mut windowed: Vec<csv::StringRecord> = Vec::new();
    let mut errors: Vec<ParseError> = Vec::new();
    let mut total_rows: u64 = 0;
    let mut widest = 0usize;

    for result in records {
        total_rows += 1;
        match result {
            Ok(rec) => {
                let data_idx = (total_rows - 1) as usize;
                let in_window = data_idx >= opts.offset && window_hi.is_none_or(|hi| data_idx < hi);
                if opts.count_only {
                    widest = widest.max(rec.len());
                } else if in_window {
                    widest = widest.max(rec.len());
                    windowed.push(rec);
                }
            }
            Err(e) => errors.push(ParseError {
                row: base_row + total_rows,
                message: e.to_string(),
            }),
        }
    }

    // Type the buffered cells in parallel.
    let data: Vec<Vec<Value>> = windowed
        .par_iter()
        .map(|rec| rec.iter().map(parse_cell).collect())
        .collect();

    // if no headers, generate A/B/C... based on the widest row seen
    let columns = if columns.is_empty() {
        (0..widest).map(col_letter).collect()
    } else {
        columns
    };
    let column_count = columns.len() as u32;

    Ok(ParsedFile {
        format: FileFormat::Csv,
        stats: ParseStats {
            total_rows,
            returned_rows: data.len() as u64,
            columns: column_count,
            elapsed_ms: start.elapsed().as_millis(),
        },
        columns,
        data,
        errors,
    })
}

/// Heuristic: count occurrences of common delimiters in the first 4KB.
fn detect_delimiter(bytes: &[u8]) -> u8 {
    let sample = &bytes[..bytes.len().min(4096)];
    let counts = [
        (b',', sample.iter().filter(|&&b| b == b',').count()),
        (b';', sample.iter().filter(|&&b| b == b';').count()),
        (b'\t', sample.iter().filter(|&&b| b == b'\t').count()),
        (b'|', sample.iter().filter(|&&b| b == b'|').count()),
    ];
    counts
        .iter()
        .max_by_key(|(_, c)| *c)
        .map(|(d, _)| *d)
        .unwrap_or(b',')
}

fn parse_cell(s: &str) -> Value {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Value::Null;
    }
    if let Ok(i) = trimmed.parse::<i64>() {
        return Value::Number(i.into());
    }
    if let Ok(f) = trimmed.parse::<f64>()
        && let Some(n) = serde_json::Number::from_f64(f)
    {
        return Value::Number(n);
    }
    match trimmed.to_lowercase().as_str() {
        "true" | "yes" | "1" => return Value::Bool(true),
        "false" | "no" | "0" => return Value::Bool(false),
        _ => {}
    }
    Value::String(s.to_string())
}

fn col_letter(i: usize) -> String {
    let mut n = i;
    let mut s = String::new();
    loop {
        s.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    s
}

fn empty_result(_opts: &ParseOptions, start: Instant) -> ParsedFile {
    ParsedFile {
        format: FileFormat::Csv,
        columns: vec![],
        data: vec![],
        stats: ParseStats {
            total_rows: 0,
            returned_rows: 0,
            columns: 0,
            elapsed_ms: start.elapsed().as_millis(),
        },
        errors: vec![],
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::domain::processing::entities::ParseOptions;

    fn opts() -> ParseOptions {
        ParseOptions::default()
    }

    #[test]
    fn headers_and_typed_cells() {
        let out = parse(b"a,b,c\n1,2.5,hello\n4,x,\n", &opts()).unwrap();
        assert_eq!(out.columns, ["a", "b", "c"]);
        assert_eq!(out.stats.total_rows, 2);
        assert_eq!(out.data[0][0], serde_json::json!(1));
        assert_eq!(out.data[0][1], serde_json::json!(2.5));
        assert_eq!(out.data[0][2], serde_json::json!("hello"));
        assert_eq!(out.data[1][2], serde_json::Value::Null);
    }

    #[test]
    fn delimiter_autodetect_semicolon() {
        let out = parse(b"a;b\n1;2\n3;4\n", &opts()).unwrap();
        assert_eq!(out.columns, ["a", "b"]);
        assert_eq!(out.stats.total_rows, 2);
    }

    #[test]
    fn pagination_offset_and_max_rows() {
        let mut o = opts();
        o.offset = 1;
        o.max_rows = Some(2);
        let out = parse(b"h\n1\n2\n3\n4\n", &o).unwrap();
        assert_eq!(out.stats.total_rows, 4);
        assert_eq!(out.stats.returned_rows, 2);
        assert_eq!(out.data[0][0], serde_json::json!(2));
    }

    #[test]
    fn no_headers_generates_letters() {
        let mut o = opts();
        o.has_headers = false;
        let out = parse(b"1,2,3\n4,5,6\n", &o).unwrap();
        assert_eq!(out.columns, ["A", "B", "C"]);
        assert_eq!(out.stats.total_rows, 2);
    }
}
