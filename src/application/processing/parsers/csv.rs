use std::time::Instant;

use serde_json::Value;

use crate::domain::processing::entities::{FileFormat, ParseError, ParseOptions, ParseStats, ParsedFile};
use crate::errors::{AppError, AppResult};

pub fn parse(bytes: &[u8], opts: &ParseOptions) -> AppResult<ParsedFile> {
    let start = Instant::now();

    let delimiter = opts.delimiter
        .map(|c| c as u8)
        .unwrap_or_else(|| detect_delimiter(bytes));

    let mut builder = csv::ReaderBuilder::new();
    builder.delimiter(delimiter).has_headers(false).flexible(true);

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

    let mut data: Vec<Vec<Value>> = Vec::new();
    let mut errors: Vec<ParseError> = Vec::new();
    let mut total_rows: u64 = 0;
    let mut row_num: u64 = opts.skip_rows as u64 + if opts.has_headers { 1 } else { 0 };

    for result in records {
        row_num += 1;
        total_rows += 1;

        if total_rows <= opts.offset as u64 {
            continue;
        }
        if let Some(max) = opts.max_rows
            && data.len() >= max
        {
            continue;
        }

        match result {
            Ok(rec) => {
                let cells: Vec<Value> = rec.iter().map(parse_cell).collect();
                data.push(cells);
            }
            Err(e) => {
                errors.push(ParseError { row: row_num, message: e.to_string() });
            }
        }
    }

    // if no headers, generate A/B/C... based on widest row
    let columns = if columns.is_empty() {
        let width = data.iter().map(|r| r.len()).max().unwrap_or(0);
        (0..width).map(col_letter).collect()
    } else {
        columns
    };

    Ok(ParsedFile {
        format: FileFormat::Csv,
        columns: columns.clone(),
        stats: ParseStats {
            total_rows,
            returned_rows: data.len() as u64,
            columns: columns.len() as u32,
            elapsed_ms: start.elapsed().as_millis(),
        },
        data,
        errors,
    })
}

/// Heuristic: count occurrences of common delimiters in the first 4KB.
fn detect_delimiter(bytes: &[u8]) -> u8 {
    let sample = &bytes[..bytes.len().min(4096)];
    let counts = [
        (b',',  sample.iter().filter(|&&b| b == b',').count()),
        (b';',  sample.iter().filter(|&&b| b == b';').count()),
        (b'\t', sample.iter().filter(|&&b| b == b'\t').count()),
        (b'|',  sample.iter().filter(|&&b| b == b'|').count()),
    ];
    counts.iter().max_by_key(|(_, c)| *c).map(|(d, _)| *d).unwrap_or(b',')
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
        if n < 26 { break; }
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
