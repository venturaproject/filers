//! Format conversion — a parsed file re-serialised as CSV / JSON / NDJSON.

use serde::Deserialize;
use serde_json::{Map, Value};

use crate::domain::processing::entities::ParsedFile;
use crate::errors::{AppError, AppResult};

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    Csv,
    Json,
    Ndjson,
}

impl Target {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "csv" => Some(Self::Csv),
            "json" => Some(Self::Json),
            "ndjson" | "jsonl" => Some(Self::Ndjson),
            _ => None,
        }
    }

    pub fn content_type(self) -> &'static str {
        match self {
            Target::Csv => "text/csv; charset=utf-8",
            Target::Json => "application/json",
            Target::Ndjson => "application/x-ndjson",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            Target::Csv => "csv",
            Target::Json => "json",
            Target::Ndjson => "ndjson",
        }
    }
}

/// Serialise `parsed` into `target`. `delimiter` applies to CSV output only.
pub fn run(parsed: &ParsedFile, target: Target, delimiter: u8) -> AppResult<Vec<u8>> {
    let headers = effective_headers(parsed);
    match target {
        Target::Csv => to_csv(parsed, &headers, delimiter),
        Target::Json => to_json(parsed, &headers),
        Target::Ndjson => to_ndjson(parsed, &headers),
    }
}

fn effective_headers(parsed: &ParsedFile) -> Vec<String> {
    if !parsed.columns.is_empty() {
        return parsed.columns.clone();
    }
    let width = parsed.data.iter().map(|r| r.len()).max().unwrap_or(0);
    (0..width).map(col_letter).collect()
}

fn to_csv(parsed: &ParsedFile, headers: &[String], delimiter: u8) -> AppResult<Vec<u8>> {
    let mut w = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .from_writer(Vec::new());

    w.write_record(headers)
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    for row in &parsed.data {
        let fields = (0..headers.len()).map(|i| cell_to_csv(row.get(i)));
        w.write_record(fields)
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
    }
    w.into_inner()
        .map_err(|e| AppError::Internal(anyhow::anyhow!(e.to_string())))
}

fn to_json(parsed: &ParsedFile, headers: &[String]) -> AppResult<Vec<u8>> {
    let rows: Vec<Value> = parsed
        .data
        .iter()
        .map(|row| Value::Object(row_object(row, headers)))
        .collect();
    serde_json::to_vec(&rows).map_err(|e| AppError::Internal(anyhow::anyhow!(e)))
}

fn to_ndjson(parsed: &ParsedFile, headers: &[String]) -> AppResult<Vec<u8>> {
    let mut out = Vec::with_capacity(parsed.data.len() * 64);
    for row in &parsed.data {
        let obj = Value::Object(row_object(row, headers));
        serde_json::to_writer(&mut out, &obj)
            .map_err(|e| AppError::Internal(anyhow::anyhow!(e)))?;
        out.push(b'\n');
    }
    Ok(out)
}

fn row_object(row: &[Value], headers: &[String]) -> Map<String, Value> {
    headers
        .iter()
        .enumerate()
        .map(|(i, h)| (h.clone(), row.get(i).cloned().unwrap_or(Value::Null)))
        .collect()
}

fn cell_to_csv(v: Option<&Value>) -> String {
    match v {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(other) => other.to_string(),
    }
}

/// `0 -> A`, `26 -> AA`, … (spreadsheet column names).
pub fn col_letter(i: usize) -> String {
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
