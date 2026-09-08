use std::io::Cursor;
use std::time::Instant;

use calamine::{Data, DataType, Reader, open_workbook_auto_from_rs};
use serde_json::Value;

use crate::domain::processing::entities::{FileFormat, ParseError, ParseOptions, ParseStats, ParsedFile};
use crate::errors::{AppError, AppResult};

pub fn parse(bytes: &[u8], format: FileFormat, opts: &ParseOptions) -> AppResult<ParsedFile> {
    let start = Instant::now();
    let cursor = Cursor::new(bytes);

    let mut workbook = open_workbook_auto_from_rs(cursor)
        .map_err(|e| AppError::ParseError(format!("Cannot open workbook: {e}")))?;

    let sheet_names = workbook.sheet_names().to_owned();
    let sheet_name = sheet_names
        .get(opts.sheet)
        .ok_or_else(|| AppError::BadRequest(format!("Sheet {} not found", opts.sheet)))?
        .clone();

    let range = workbook
        .worksheet_range(&sheet_name)
        .map_err(|e| AppError::ParseError(format!("Cannot read sheet: {e}")))?;

    let mut rows = range.rows();

    // skip leading rows
    for _ in 0..opts.skip_rows {
        rows.next();
    }

    // headers
    let columns: Vec<String> = if opts.has_headers {
        match rows.next() {
            Some(row) => row.iter().map(cell_to_header).collect(),
            None => return Ok(empty_result(format, start)),
        }
    } else {
        (0..range.width()).map(col_letter).collect()
    };

    let mut data: Vec<Vec<Value>> = Vec::new();
    let mut errors: Vec<ParseError> = Vec::new();
    let mut total_rows: u64 = 0;
    let mut row_num: u64 = opts.skip_rows as u64 + if opts.has_headers { 1 } else { 0 };

    for row in rows {
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

        let cells: Vec<Value> = row.iter().map(|c| cell_to_value(c, row_num, &mut errors)).collect();
        data.push(cells);
    }

    Ok(ParsedFile {
        format,
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

fn cell_to_value(cell: &Data, row: u64, errors: &mut Vec<ParseError>) -> Value {
    match cell {
        Data::Empty => Value::Null,
        Data::String(s) => Value::String(s.clone()),
        Data::Float(f) => serde_json::Number::from_f64(*f)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        Data::Int(i) => Value::Number((*i).into()),
        Data::Bool(b) => Value::Bool(*b),
        Data::DateTime(_) | Data::DateTimeIso(_) | Data::DurationIso(_) => {
            if let Some(dt) = cell.as_datetime() {
                Value::String(dt.to_string())
            } else if let Some(s) = cell.get_string() {
                Value::String(s.to_string())
            } else {
                Value::Null
            }
        }
        Data::Error(e) => {
            errors.push(ParseError { row, message: format!("Cell error: {e:?}") });
            Value::Null
        }
    }
}

fn cell_to_header(cell: &Data) -> String {
    match cell {
        Data::String(s) if !s.trim().is_empty() => s.trim().to_string(),
        Data::Float(f) => f.to_string(),
        Data::Int(i) => i.to_string(),
        _ => String::new(),
    }
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

fn empty_result(format: FileFormat, start: Instant) -> ParsedFile {
    ParsedFile {
        format,
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
