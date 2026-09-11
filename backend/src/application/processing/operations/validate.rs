//! Row-level validation of a parsed file against a column schema.

use std::collections::HashSet;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::processing::entities::ParsedFile;
use crate::errors::{AppError, AppResult};

const DEFAULT_MAX_ERRORS: usize = 1_000;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Integer,
    Float,
    Number,
    Boolean,
}

#[derive(Debug, Deserialize)]
pub struct ColumnRule {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub unique: bool,
    #[serde(rename = "type")]
    pub ty: Option<FieldType>,
    pub regex: Option<String>,
    #[serde(rename = "enum")]
    pub allowed: Option<Vec<String>>,
    /// Numeric bounds (inclusive) for numeric `type`s.
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// Length bounds for string values.
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct Schema {
    /// `column name -> rules`.
    pub columns: std::collections::HashMap<String, ColumnRule>,
    /// Reject rows/files that carry columns not in the schema.
    #[serde(default = "default_true")]
    pub allow_extra_columns: bool,
    pub max_errors: Option<usize>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
pub struct RowError {
    pub row: u64,
    pub column: String,
    pub rule: &'static str,
    pub value: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ValidationReport {
    pub valid: bool,
    pub total_rows: u64,
    pub error_count: u64,
    pub errors_truncated: bool,
    pub missing_columns: Vec<String>,
    pub unexpected_columns: Vec<String>,
    pub errors: Vec<RowError>,
}

struct CompiledRule<'a> {
    name: &'a str,
    idx: Option<usize>,
    rule: &'a ColumnRule,
    regex: Option<Regex>,
    seen: HashSet<String>,
}

pub fn run(parsed: &ParsedFile, schema: &Schema) -> AppResult<ValidationReport> {
    let max_errors = schema.max_errors.unwrap_or(DEFAULT_MAX_ERRORS).min(50_000);

    let missing_columns: Vec<String> = schema
        .columns
        .keys()
        .filter(|c| !parsed.columns.iter().any(|h| h == *c))
        .cloned()
        .collect();

    let unexpected_columns: Vec<String> = if schema.allow_extra_columns {
        Vec::new()
    } else {
        parsed
            .columns
            .iter()
            .filter(|h| !schema.columns.contains_key(*h))
            .cloned()
            .collect()
    };

    let mut rules: Vec<CompiledRule> = Vec::with_capacity(schema.columns.len());
    for (name, rule) in &schema.columns {
        let regex = match &rule.regex {
            Some(p) => Some(
                Regex::new(p)
                    .map_err(|e| AppError::BadRequest(format!("bad regex for '{name}': {e}")))?,
            ),
            None => None,
        };
        rules.push(CompiledRule {
            name,
            idx: parsed.columns.iter().position(|h| h == name),
            rule,
            regex,
            seen: HashSet::new(),
        });
    }

    let mut errors: Vec<RowError> = Vec::new();
    let mut error_count = 0u64;
    let mut truncated = false;

    for (r, row) in parsed.data.iter().enumerate() {
        let row_no = r as u64 + 1;
        for cr in &mut rules {
            let cell = cr.idx.and_then(|i| row.get(i));
            for (rule_name, msg) in check_cell(cr, cell) {
                error_count += 1;
                if errors.len() < max_errors {
                    errors.push(RowError {
                        row: row_no,
                        column: cr.name.to_string(),
                        rule: rule_name,
                        value: cell.map(display).unwrap_or_default(),
                        message: msg,
                    });
                } else {
                    truncated = true;
                }
            }
        }
    }

    Ok(ValidationReport {
        valid: error_count == 0 && missing_columns.is_empty() && unexpected_columns.is_empty(),
        total_rows: parsed.data.len() as u64,
        error_count,
        errors_truncated: truncated,
        missing_columns,
        unexpected_columns,
        errors,
    })
}

fn is_blank(cell: Option<&Value>) -> bool {
    match cell {
        None | Some(Value::Null) => true,
        Some(Value::String(s)) => s.trim().is_empty(),
        _ => false,
    }
}

fn check_cell(cr: &mut CompiledRule, cell: Option<&Value>) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    let rule = cr.rule;

    let cell = match cell {
        c if is_blank(c) => {
            if rule.required {
                out.push(("required", "value is required".into()));
            }
            return out; // no other rule applies to an absent value
        }
        Some(v) => v,
        None => return out,
    };
    let text = display(cell);

    if let Some(ty) = rule.ty
        && !type_ok(ty, cell)
    {
        out.push(("type", format!("expected {ty:?} value")));
    }

    if let Some(num) = as_number(cell) {
        if let Some(min) = rule.min
            && num < min
        {
            out.push(("min", format!("{num} < {min}")));
        }
        if let Some(max) = rule.max
            && num > max
        {
            out.push(("max", format!("{num} > {max}")));
        }
    }

    if let Value::String(s) = cell {
        if let Some(minl) = rule.min_length
            && s.chars().count() < minl
        {
            out.push(("min_length", format!("shorter than {minl} chars")));
        }
        if let Some(maxl) = rule.max_length
            && s.chars().count() > maxl
        {
            out.push(("max_length", format!("longer than {maxl} chars")));
        }
    }

    if let Some(re) = &cr.regex
        && !re.is_match(&text)
    {
        out.push(("regex", format!("does not match {}", re.as_str())));
    }

    if let Some(allowed) = &rule.allowed
        && !allowed.iter().any(|a| a == &text)
    {
        out.push(("enum", "value not in the allowed set".into()));
    }

    if rule.unique && !cr.seen.insert(text.clone()) {
        out.push(("unique", "duplicate value".into()));
    }

    out
}

fn type_ok(ty: FieldType, v: &Value) -> bool {
    match ty {
        FieldType::String => matches!(v, Value::String(_)),
        FieldType::Boolean => matches!(v, Value::Bool(_)),
        FieldType::Integer => v.as_i64().is_some() || v.as_u64().is_some(),
        FieldType::Float => matches!(v, Value::Number(n) if n.as_f64().is_some()),
        FieldType::Number => matches!(v, Value::Number(_)),
    }
}

fn as_number(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn display(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}
