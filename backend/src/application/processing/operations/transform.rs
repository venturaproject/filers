//! Declarative row/column transformation of a parsed file.
//!
//! Order of operations: `cast` → `filter` → `select`/`drop` → `rename` →
//! `offset`/`limit`.

use std::collections::HashMap;

use regex::Regex;
use serde::Deserialize;
use serde_json::{Number, Value};

use crate::domain::processing::entities::{ParseStats, ParsedFile};
use crate::errors::{AppError, AppResult};

#[derive(Debug, Clone, Deserialize)]
pub struct Spec {
    /// Keep only these columns, in this order. Omit to keep all.
    #[serde(default)]
    pub select: Vec<String>,
    /// Remove these columns (applied when `select` is empty).
    #[serde(default)]
    pub drop: Vec<String>,
    /// `old name -> new name`, applied after select/drop.
    #[serde(default)]
    pub rename: HashMap<String, String>,
    /// Coerce a column to a type: `integer` | `float` | `string` | `boolean`.
    #[serde(default)]
    pub cast: HashMap<String, CastType>,
    /// Keep rows where **every** column predicate matches.
    #[serde(default)]
    pub filter: HashMap<String, Predicate>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CastType {
    Integer,
    Float,
    String,
    Boolean,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Predicate {
    pub eq: Option<Value>,
    pub ne: Option<Value>,
    pub gt: Option<f64>,
    pub gte: Option<f64>,
    pub lt: Option<f64>,
    pub lte: Option<f64>,
    #[serde(rename = "in")]
    pub is_in: Option<Vec<Value>>,
    pub not_in: Option<Vec<Value>>,
    /// Regex (or plain substring) match on the stringified value.
    pub matches: Option<String>,
    pub is_null: Option<bool>,
    pub not_null: Option<bool>,
}

/// The transformed file plus how many rows passed `filter` before
/// `offset`/`limit` were applied.
pub struct Transformed {
    pub file: ParsedFile,
    pub matched_rows: u64,
}

pub fn run(parsed: &ParsedFile, spec: &Spec, start: std::time::Instant) -> AppResult<Transformed> {
    let mut columns = parsed.columns.clone();
    let width = columns
        .len()
        .max(parsed.data.first().map_or(0, |r| r.len()));
    while columns.len() < width {
        columns.push(super::convert::col_letter(columns.len()));
    }
    let col_idx: HashMap<&str, usize> = columns
        .iter()
        .enumerate()
        .map(|(i, c)| (c.as_str(), i))
        .collect();

    // ── compile filters ────────────────────────────────────────────────────
    let mut filters: Vec<(usize, &Predicate, Option<Regex>)> = Vec::new();
    for (name, pred) in &spec.filter {
        let idx = *col_idx.get(name.as_str()).ok_or_else(|| {
            AppError::BadRequest(format!("filter references unknown column '{name}'"))
        })?;
        let re = match &pred.matches {
            Some(p) => Some(
                Regex::new(p)
                    .map_err(|e| AppError::BadRequest(format!("bad regex for '{name}': {e}")))?,
            ),
            None => None,
        };
        filters.push((idx, pred, re));
    }

    let casts: Vec<(usize, CastType)> = spec
        .cast
        .iter()
        .filter_map(|(name, ty)| col_idx.get(name.as_str()).map(|i| (*i, *ty)))
        .collect();

    // ── cast + filter rows ─────────────────────────────────────────────────
    let total_rows = parsed.data.len() as u64;
    let mut rows: Vec<Vec<Value>> = Vec::new();
    for row in &parsed.data {
        let mut row: Vec<Value> = {
            let mut r = row.clone();
            r.resize(width, Value::Null);
            r
        };
        for (i, ty) in &casts {
            row[*i] = cast(&row[*i], *ty);
        }
        if filters
            .iter()
            .all(|(i, pred, re)| matches_predicate(&row[*i], pred, re.as_ref()))
        {
            rows.push(row);
        }
    }

    // ── column projection ──────────────────────────────────────────────────
    let keep: Vec<usize> = if !spec.select.is_empty() {
        spec.select
            .iter()
            .map(|name| {
                col_idx.get(name.as_str()).copied().ok_or_else(|| {
                    AppError::BadRequest(format!("select references unknown column '{name}'"))
                })
            })
            .collect::<AppResult<_>>()?
    } else if !spec.drop.is_empty() {
        let drop: std::collections::HashSet<&str> = spec.drop.iter().map(String::as_str).collect();
        (0..width)
            .filter(|i| !drop.contains(columns[*i].as_str()))
            .collect()
    } else {
        (0..width).collect()
    };

    let mut out_cols: Vec<String> = keep.iter().map(|i| columns[*i].clone()).collect();
    for c in &mut out_cols {
        if let Some(new) = spec.rename.get(c) {
            *c = new.clone();
        }
    }

    let mut projected: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| keep.iter().map(|i| row[*i].clone()).collect())
        .collect();

    // ── offset / limit ─────────────────────────────────────────────────────
    let matched = projected.len() as u64;
    let lo = spec.offset.unwrap_or(0).min(projected.len());
    projected.drain(..lo);
    if let Some(lim) = spec.limit {
        projected.truncate(lim);
    }

    let mut timings = parsed.timings.clone();
    timings.convert_ms = start.elapsed().as_millis();
    timings.total_ms = timings.upload_ms + timings.parse_ms + timings.convert_ms;

    Ok(Transformed {
        file: ParsedFile {
            format: parsed.format.clone(),
            stats: ParseStats {
                total_rows,
                returned_rows: projected.len() as u64,
                columns: out_cols.len() as u32,
                elapsed_ms: timings.parse_ms,
            },
            columns: out_cols,
            data: projected,
            errors: vec![],
            timings,
        },
        matched_rows: matched,
    })
}

fn cast(v: &Value, ty: CastType) -> Value {
    let s = match v {
        Value::Null => return Value::Null,
        Value::String(s) => s.trim().to_string(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    };
    if s.is_empty() {
        return Value::Null;
    }
    match ty {
        CastType::Integer => s
            .parse::<i64>()
            .map(Value::from)
            .unwrap_or(Value::String(s)),
        CastType::Float => s
            .parse::<f64>()
            .ok()
            .and_then(Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::String(s)),
        CastType::String => Value::String(s),
        CastType::Boolean => match s.to_lowercase().as_str() {
            "true" | "1" | "yes" | "y" => Value::Bool(true),
            "false" | "0" | "no" | "n" => Value::Bool(false),
            _ => Value::String(s),
        },
    }
}

fn as_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

fn stringify(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn matches_predicate(v: &Value, p: &Predicate, re: Option<&Regex>) -> bool {
    let is_null = matches!(v, Value::Null) || matches!(v, Value::String(s) if s.trim().is_empty());

    if let Some(want) = p.is_null
        && is_null != want
    {
        return false;
    }
    if let Some(want) = p.not_null
        && is_null == want
    {
        return false;
    }
    if let Some(eq) = &p.eq
        && stringify(v) != stringify(eq)
    {
        return false;
    }
    if let Some(ne) = &p.ne
        && stringify(v) == stringify(ne)
    {
        return false;
    }
    if let Some(list) = &p.is_in
        && !list.iter().any(|x| stringify(x) == stringify(v))
    {
        return false;
    }
    if let Some(list) = &p.not_in
        && list.iter().any(|x| stringify(x) == stringify(v))
    {
        return false;
    }
    for (bound, cmp) in [
        (p.gt, f64::gt as fn(&f64, &f64) -> bool),
        (p.gte, f64::ge),
        (p.lt, f64::lt),
        (p.lte, f64::le),
    ] {
        if let Some(b) = bound {
            match as_f64(v) {
                Some(n) if cmp(&n, &b) => {}
                _ => return false,
            }
        }
    }
    if let Some(re) = re
        && !re.is_match(&stringify(v))
    {
        return false;
    }
    true
}
