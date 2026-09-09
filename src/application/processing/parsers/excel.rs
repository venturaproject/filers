use std::io::Cursor;
use std::time::Instant;

use calamine::{CellType, DataType, Range, Reader, ReaderRef, open_workbook_auto_from_rs};
use rayon::prelude::*;
use serde_json::Value;

use crate::application::processing::timing::process_cpu_time;
use crate::domain::processing::entities::{
    FileFormat, ParseError, ParseOptions, ParseStats, ParsedFile, Timings,
};
use crate::errors::{AppError, AppResult};

pub fn parse(bytes: &[u8], format: FileFormat, opts: &ParseOptions) -> AppResult<ParsedFile> {
    let start = Instant::now();
    let cpu_start = process_cpu_time();

    let t_open = Instant::now();
    let mut workbook = open_workbook_auto_from_rs(Cursor::new(bytes))
        .map_err(|e| AppError::ParseError(format!("Cannot open workbook: {e}")))?;

    let sheet_names = workbook.sheet_names().to_owned();
    let sheet_name = sheet_names
        .get(opts.sheet)
        .ok_or_else(|| AppError::BadRequest(format!("Sheet {} not found", opts.sheet)))?
        .clone();
    let open_ms = t_open.elapsed().as_millis();

    // `xlsx`/`xlsb` expose a borrowed `Range<DataRef>` — cells point straight at
    // the shared-string table instead of `worksheet_range` cloning all 1M+ of
    // them into owned `String`s first. `xls`/`ods` only have the owned path.
    let t_read = Instant::now();
    let mut timings = Timings {
        open_ms,
        ..Default::default()
    };
    match format {
        FileFormat::Xlsx => {
            let range = workbook
                .worksheet_range_ref(&sheet_name)
                .map_err(|e| AppError::ParseError(format!("Cannot read sheet: {e}")))?;
            timings.read_ms = t_read.elapsed().as_millis();
            Ok(assemble(&range, format, opts, start, cpu_start, timings))
        }
        // `xls` / `ods` only expose the owned `Range<Data>` path.
        _ => {
            let range = workbook
                .worksheet_range(&sheet_name)
                .map_err(|e| AppError::ParseError(format!("Cannot read sheet: {e}")))?;
            timings.read_ms = t_read.elapsed().as_millis();
            Ok(assemble(&range, format, opts, start, cpu_start, timings))
        }
    }
}

/// Shared row-processing over either `Range<Data>` (owned) or `Range<DataRef>`
/// (borrowed) — the only difference the caller sees is which one it hands in.
fn assemble<D: CellType + DataType + Sync>(
    range: &Range<D>,
    format: FileFormat,
    opts: &ParseOptions,
    start: Instant,
    cpu_start: std::time::Duration,
    mut timings: Timings,
) -> ParsedFile {
    let width = range.width();
    let all_rows: Vec<&[D]> = range.rows().collect();

    // Advance past the skipped leading rows.
    let mut idx = opts.skip_rows.min(all_rows.len());

    // Header row (or synthetic A/B/C… names).
    let columns: Vec<String> = if opts.has_headers {
        match all_rows.get(idx) {
            Some(row) => {
                idx += 1;
                row.iter().map(cell_to_header).collect()
            }
            None => return empty_result(format, start),
        }
    } else {
        (0..width).map(col_letter).collect()
    };
    let column_count = columns.len() as u32;

    // Rows consumed before the body — used to reconstruct 1-based row numbers.
    let header_offset = idx as u64;
    let body = &all_rows[idx..];
    let total_rows = body.len() as u64;

    // Batch jobs only keep the stats, so skip building `data` entirely.
    let t_convert = Instant::now();
    let (data, errors) = if opts.count_only {
        (Vec::new(), Vec::new())
    } else {
        let lo = opts.offset.min(body.len());
        let hi = match opts.max_rows {
            Some(max) => lo.saturating_add(max).min(body.len()),
            None => body.len(),
        };
        convert_rows(&body[lo..hi], header_offset + lo as u64)
    };
    timings.convert_ms = t_convert.elapsed().as_millis();
    timings.parse_ms = start.elapsed().as_millis();
    timings.parse_cpu_ms = process_cpu_time().saturating_sub(cpu_start).as_millis();

    ParsedFile {
        format,
        stats: ParseStats {
            total_rows,
            returned_rows: data.len() as u64,
            columns: column_count,
            elapsed_ms: timings.parse_ms,
        },
        columns,
        data,
        errors,
        timings,
    }
}

/// Convert a window of spreadsheet rows into JSON cells, in parallel.
///
/// `first_row_offset` is the count of rows before `window[0]` (skipped rows +
/// header), so cell-error messages can report a correct 1-based row number.
fn convert_rows<D: DataType + Sync>(
    window: &[&[D]],
    first_row_offset: u64,
) -> (Vec<Vec<Value>>, Vec<ParseError>) {
    let converted: Vec<(Vec<Value>, Vec<ParseError>)> = window
        .par_iter()
        .enumerate()
        .map(|(i, row)| {
            let row_num = first_row_offset + i as u64 + 1;
            let mut errs = Vec::new();
            let cells = row
                .iter()
                .map(|c| cell_to_value(c, row_num, &mut errs))
                .collect();
            (cells, errs)
        })
        .collect();

    let mut data = Vec::with_capacity(converted.len());
    let mut errors = Vec::new();
    for (cells, errs) in converted {
        data.push(cells);
        if !errs.is_empty() {
            errors.extend(errs);
        }
    }
    (data, errors)
}

fn cell_to_value<D: DataType>(cell: &D, row: u64, errors: &mut Vec<ParseError>) -> Value {
    if cell.is_empty() {
        return Value::Null;
    }
    if cell.is_datetime() || cell.is_datetime_iso() || cell.is_duration_iso() {
        return match cell.as_datetime() {
            Some(dt) => Value::String(dt.to_string()),
            None => match cell.get_string() {
                Some(s) => Value::String(s.to_string()),
                None => Value::Null,
            },
        };
    }
    if let Some(s) = cell.get_string() {
        return Value::String(s.to_string());
    }
    if let Some(i) = cell.get_int() {
        return Value::Number(i.into());
    }
    if let Some(f) = cell.get_float() {
        return serde_json::Number::from_f64(f)
            .map(Value::Number)
            .unwrap_or(Value::Null);
    }
    if let Some(b) = cell.get_bool() {
        return Value::Bool(b);
    }
    if let Some(e) = cell.get_error() {
        errors.push(ParseError {
            row,
            message: format!("Cell error: {e:?}"),
        });
    }
    Value::Null
}

fn cell_to_header<D: DataType>(cell: &D) -> String {
    if let Some(s) = cell.get_string() {
        return s.trim().to_string();
    }
    if let Some(i) = cell.get_int() {
        return i.to_string();
    }
    if let Some(f) = cell.get_float() {
        return f.to_string();
    }
    String::new()
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
        timings: Timings {
            parse_ms: start.elapsed().as_millis(),
            ..Default::default()
        },
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::domain::processing::entities::ParseOptions;

    fn fixture() -> Vec<u8> {
        std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/ventas.xlsx"
        ))
        .expect("fixture")
    }

    #[test]
    fn reads_the_sample_sheet() {
        let out = parse(&fixture(), FileFormat::Xlsx, &ParseOptions::default()).unwrap();
        assert!(matches!(out.format, FileFormat::Xlsx));
        assert!(!out.columns.is_empty());
        assert!(out.stats.total_rows >= 1);
        assert_eq!(out.stats.columns as usize, out.columns.len());
    }

    #[test]
    fn sheet_out_of_range_errors() {
        let mut o = ParseOptions::default();
        o.sheet = 9;
        let err = parse(&fixture(), FileFormat::Xlsx, &o).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("sheet"));
    }
}
