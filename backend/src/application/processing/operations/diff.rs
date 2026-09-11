//! Row-level diff of two parsed files, matched on one or more key columns.

use std::collections::HashMap;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::domain::processing::entities::ParsedFile;
use crate::errors::{AppError, AppResult};

/// Cap on the number of detailed rows returned per bucket. The summary counts
/// stay exact.
const DETAIL_CAP: usize = 5_000;

#[derive(Debug, Serialize)]
pub struct FieldChange {
    pub from: Value,
    pub to: Value,
}

#[derive(Debug, Serialize)]
pub struct ChangedRow {
    pub key: Map<String, Value>,
    pub changes: Map<String, Value>,
}

#[derive(Debug, Serialize)]
pub struct KeyedRow {
    pub key: Map<String, Value>,
    pub row: Map<String, Value>,
}

#[derive(Debug, Serialize)]
pub struct DiffSummary {
    pub added: u64,
    pub removed: u64,
    pub changed: u64,
    pub unchanged: u64,
    pub columns_added: Vec<String>,
    pub columns_removed: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct DiffReport {
    pub key: Vec<String>,
    pub summary: DiffSummary,
    pub truncated: bool,
    pub added: Vec<KeyedRow>,
    pub removed: Vec<KeyedRow>,
    pub changed: Vec<ChangedRow>,
}

pub fn run(a: &ParsedFile, b: &ParsedFile, key_cols: &[String]) -> AppResult<DiffReport> {
    if key_cols.is_empty() {
        return Err(AppError::BadRequest(
            "`key` is required — one or more column names to match rows on".into(),
        ));
    }
    for k in key_cols {
        if !a.columns.iter().any(|c| c == k) || !b.columns.iter().any(|c| c == k) {
            return Err(AppError::BadRequest(format!(
                "key column '{k}' is not present in both files"
            )));
        }
    }

    let a_idx: HashMap<&str, usize> = index(&a.columns);
    let b_idx: HashMap<&str, usize> = index(&b.columns);

    let columns_added: Vec<String> = b
        .columns
        .iter()
        .filter(|c| !a_idx.contains_key(c.as_str()))
        .cloned()
        .collect();
    let columns_removed: Vec<String> = a
        .columns
        .iter()
        .filter(|c| !b_idx.contains_key(c.as_str()))
        .cloned()
        .collect();

    // Columns compared for "changed": present in both, excluding the key.
    let common: Vec<String> = a
        .columns
        .iter()
        .filter(|c| b_idx.contains_key(c.as_str()) && !key_cols.contains(c))
        .cloned()
        .collect();

    let a_rows = index_rows(a, &a_idx, key_cols);
    let mut b_rows = index_rows(b, &b_idx, key_cols);

    let mut summary = DiffSummary {
        added: 0,
        removed: 0,
        changed: 0,
        unchanged: 0,
        columns_added,
        columns_removed,
    };
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();
    let mut truncated = false;

    for (key, a_row) in &a_rows {
        match b_rows.remove(key) {
            None => {
                summary.removed += 1;
                push_capped(
                    &mut removed,
                    &mut truncated,
                    KeyedRow {
                        key: key_map(key_cols, a_row, &a_idx),
                        row: row_map(&a.columns, a_row),
                    },
                );
            }
            Some(b_row) => {
                let mut changes = Map::new();
                for col in &common {
                    let av = cell(a_row, &a_idx, col);
                    let bv = cell(b_row, &b_idx, col);
                    if !values_equal(av, bv) {
                        changes.insert(
                            col.clone(),
                            serde_json::to_value(FieldChange {
                                from: av.clone(),
                                to: bv.clone(),
                            })
                            .unwrap_or(Value::Null),
                        );
                    }
                }
                if changes.is_empty() {
                    summary.unchanged += 1;
                } else {
                    summary.changed += 1;
                    push_capped(
                        &mut changed,
                        &mut truncated,
                        ChangedRow {
                            key: key_map(key_cols, a_row, &a_idx),
                            changes,
                        },
                    );
                }
            }
        }
    }

    // Whatever is left in b was not in a.
    for (_key, b_row) in b_rows {
        summary.added += 1;
        push_capped(
            &mut added,
            &mut truncated,
            KeyedRow {
                key: key_map(key_cols, b_row, &b_idx),
                row: row_map(&b.columns, b_row),
            },
        );
    }

    Ok(DiffReport {
        key: key_cols.to_vec(),
        summary,
        truncated,
        added,
        removed,
        changed,
    })
}

fn index(cols: &[String]) -> HashMap<&str, usize> {
    cols.iter()
        .enumerate()
        .map(|(i, c)| (c.as_str(), i))
        .collect()
}

/// `composite key string -> row`. Later duplicates win (last row for a key).
fn index_rows<'a>(
    file: &'a ParsedFile,
    idx: &HashMap<&str, usize>,
    key_cols: &[String],
) -> HashMap<String, &'a [Value]> {
    let key_positions: Vec<usize> = key_cols
        .iter()
        .filter_map(|k| idx.get(k.as_str()).copied())
        .collect();
    let mut map = HashMap::with_capacity(file.data.len());
    for row in &file.data {
        let key = key_positions
            .iter()
            .map(|p| stringify(row.get(*p).unwrap_or(&Value::Null)))
            .collect::<Vec<_>>()
            .join("\u{1f}");
        map.insert(key, row.as_slice());
    }
    map
}

fn cell<'a>(row: &'a [Value], idx: &HashMap<&str, usize>, col: &str) -> &'a Value {
    idx.get(col)
        .and_then(|i| row.get(*i))
        .unwrap_or(&Value::Null)
}

fn key_map(key_cols: &[String], row: &[Value], idx: &HashMap<&str, usize>) -> Map<String, Value> {
    key_cols
        .iter()
        .map(|k| (k.clone(), cell(row, idx, k).clone()))
        .collect()
}

fn row_map(columns: &[String], row: &[Value]) -> Map<String, Value> {
    columns
        .iter()
        .enumerate()
        .map(|(i, c)| (c.clone(), row.get(i).cloned().unwrap_or(Value::Null)))
        .collect()
}

fn values_equal(a: &Value, b: &Value) -> bool {
    stringify(a) == stringify(b)
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

fn push_capped<T>(bucket: &mut Vec<T>, truncated: &mut bool, item: T) {
    if bucket.len() < DETAIL_CAP {
        bucket.push(item);
    } else {
        *truncated = true;
    }
}
