//! Build an `.xlsx` workbook from tabular data (a parsed file, or a JSON body).

use rust_xlsxwriter::{Format, Workbook};
use serde::Deserialize;
use serde_json::Value;

use crate::domain::processing::entities::ParsedFile;
use crate::errors::{AppError, AppResult};

/// Sheet-building options, shared by the JSON body and the `?to=xlsx` path.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct XlsxOptions {
    #[serde(default = "default_sheet_name")]
    pub sheet_name: String,
    #[serde(default = "default_true")]
    pub header_style: bool,
    #[serde(default = "default_true")]
    pub freeze_header: bool,
    #[serde(default)]
    pub auto_filter: bool,
}

impl Default for XlsxOptions {
    fn default() -> Self {
        Self {
            sheet_name: default_sheet_name(),
            header_style: true,
            freeze_header: true,
            auto_filter: false,
        }
    }
}

fn default_sheet_name() -> String {
    "Sheet1".into()
}
fn default_true() -> bool {
    true
}

/// JSON request body for `POST /api/generate/xlsx`.
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct XlsxRequest {
    /// Explicit column order. Optional when `rows` are objects.
    #[serde(default)]
    pub columns: Vec<String>,
    /// Rows as arrays (with `columns`) or as objects.
    pub rows: Vec<Value>,
    #[serde(default)]
    pub options: XlsxOptions,
}

pub fn from_request(req: &XlsxRequest) -> AppResult<Vec<u8>> {
    let columns = if !req.columns.is_empty() {
        req.columns.clone()
    } else {
        infer_columns(&req.rows)
    };
    let rows: Vec<Vec<Value>> = req
        .rows
        .iter()
        .map(|row| normalise_row(row, &columns))
        .collect();
    build(&columns, &rows, &req.options)
}

pub fn from_parsed(parsed: &ParsedFile, options: &XlsxOptions) -> AppResult<Vec<u8>> {
    let columns = if parsed.columns.is_empty() {
        let width = parsed.data.first().map_or(0, |r| r.len());
        (0..width).map(super::convert::col_letter).collect()
    } else {
        parsed.columns.clone()
    };
    build(&columns, &parsed.data, options)
}

fn build(columns: &[String], rows: &[Vec<Value>], opts: &XlsxOptions) -> AppResult<Vec<u8>> {
    let mut wb = Workbook::new();
    let sheet = wb.add_worksheet();
    sheet
        .set_name(&opts.sheet_name)
        .map_err(|e| AppError::BadRequest(format!("bad sheet name: {e}")))?;

    let header_fmt = Format::new().set_bold();

    for (c, name) in columns.iter().enumerate() {
        let col = c as u16;
        if opts.header_style {
            sheet.write_string_with_format(0, col, name, &header_fmt)
        } else {
            sheet.write_string(0, col, name)
        }
        .map_err(xlsx_err)?;
    }

    for (r, row) in rows.iter().enumerate() {
        let excel_row = r as u32 + 1;
        for (c, cell) in row.iter().take(columns.len()).enumerate() {
            write_cell(sheet, excel_row, c as u16, cell)?;
        }
    }

    if opts.freeze_header {
        sheet.set_freeze_panes(1, 0).map_err(xlsx_err)?;
    }
    if opts.auto_filter && !columns.is_empty() {
        let last_row = rows.len() as u32;
        let last_col = columns.len() as u16 - 1;
        sheet
            .autofilter(0, 0, last_row.max(1), last_col)
            .map_err(xlsx_err)?;
    }

    wb.save_to_buffer().map_err(xlsx_err)
}

fn write_cell(
    sheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    col: u16,
    v: &Value,
) -> AppResult<()> {
    match v {
        Value::Null => Ok(()),
        Value::Bool(b) => sheet.write_boolean(row, col, *b).map(|_| ()),
        Value::Number(n) => match n.as_f64() {
            Some(f) => sheet.write_number(row, col, f).map(|_| ()),
            None => sheet.write_string(row, col, n.to_string()).map(|_| ()),
        },
        Value::String(s) => sheet.write_string(row, col, s).map(|_| ()),
        other => sheet.write_string(row, col, other.to_string()).map(|_| ()),
    }
    .map_err(xlsx_err)
}

fn xlsx_err(e: rust_xlsxwriter::XlsxError) -> AppError {
    AppError::Internal(anyhow::anyhow!("xlsx: {e}"))
}

fn infer_columns(rows: &[Value]) -> Vec<String> {
    let mut seen = Vec::new();
    for row in rows {
        if let Value::Object(map) = row {
            for k in map.keys() {
                if !seen.iter().any(|s| s == k) {
                    seen.push(k.clone());
                }
            }
        }
    }
    seen
}

fn normalise_row(row: &Value, columns: &[String]) -> Vec<Value> {
    match row {
        Value::Array(a) => a.clone(),
        Value::Object(map) => columns
            .iter()
            .map(|c| map.get(c).cloned().unwrap_or(Value::Null))
            .collect(),
        other => vec![other.clone()],
    }
}
