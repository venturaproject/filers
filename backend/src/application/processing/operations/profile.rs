//! Column profiling — per-column type inference and summary statistics.

use std::collections::HashMap;

use rayon::prelude::*;
use serde::Serialize;
use serde_json::Value;

use crate::domain::processing::entities::ParsedFile;

/// Cardinality/frequency tracking is capped so a high-entropy column can't blow
/// up memory. Past the cap, `distinct` becomes a lower bound.
const DISTINCT_CAP: usize = 50_000;
const TOP_N: usize = 10;
const SAMPLES: usize = 5;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InferredType {
    Empty,
    Integer,
    Float,
    Boolean,
    String,
}

#[derive(Debug, Serialize)]
pub struct ValueCount {
    pub value: String,
    pub count: u64,
}

#[derive(Debug, Serialize)]
pub struct ColumnProfile {
    pub column: String,
    pub index: usize,
    pub inferred_type: InferredType,
    /// Non-null values seen.
    pub count: u64,
    pub nulls: u64,
    pub blanks: u64,
    /// Distinct non-null values; `distinct_capped` marks it as a lower bound.
    pub distinct: u64,
    pub distinct_capped: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mean: Option<f64>,
    pub top: Vec<ValueCount>,
    pub samples: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ProfileReport {
    pub columns: Vec<String>,
    pub total_rows: u64,
    pub profile: Vec<ColumnProfile>,
}

pub fn run(parsed: &ParsedFile) -> ProfileReport {
    let n_cols = parsed
        .columns
        .len()
        .max(parsed.data.first().map_or(0, |r| r.len()));

    let profile: Vec<ColumnProfile> = (0..n_cols)
        .into_par_iter()
        .map(|col| profile_column(parsed, col))
        .collect();

    ProfileReport {
        columns: parsed.columns.clone(),
        total_rows: parsed.data.len() as u64,
        profile,
    }
}

fn profile_column(parsed: &ParsedFile, col: usize) -> ColumnProfile {
    let name =
        parsed.columns.get(col).cloned().unwrap_or_else(|| {
            crate::application::processing::operations::convert::col_letter(col)
        });

    let mut count = 0u64;
    let mut nulls = 0u64;
    let mut blanks = 0u64;
    let mut is_int = true;
    let mut is_float = true;
    let mut is_bool = true;
    let mut seen_any = false;

    let mut freq: HashMap<String, u64> = HashMap::new();
    let mut freq_full = false;
    let mut samples: Vec<String> = Vec::new();

    let mut num_min = f64::INFINITY;
    let mut num_max = f64::NEG_INFINITY;
    let mut num_sum = 0.0f64;
    let mut num_n = 0u64;
    let mut str_min: Option<String> = None;
    let mut str_max: Option<String> = None;

    for row in &parsed.data {
        let cell = match row.get(col) {
            None | Some(Value::Null) => {
                nulls += 1;
                continue;
            }
            Some(Value::String(s)) if s.trim().is_empty() => {
                blanks += 1;
                nulls += 1;
                continue;
            }
            Some(v) => v,
        };
        seen_any = true;
        count += 1;

        // type inference
        match cell {
            Value::Bool(_) => {
                is_int = false;
                is_float = false;
            }
            Value::Number(nu) => {
                is_bool = false;
                if !nu.is_i64() && !nu.is_u64() {
                    is_int = false;
                }
                let f = nu.as_f64().unwrap_or(f64::NAN);
                if f.is_finite() {
                    num_min = num_min.min(f);
                    num_max = num_max.max(f);
                    num_sum += f;
                    num_n += 1;
                }
            }
            Value::String(_) => {
                is_int = false;
                is_float = false;
                is_bool = false;
            }
            _ => {
                is_int = false;
                is_float = false;
                is_bool = false;
            }
        }

        let s = value_to_string(cell);

        // string min/max (lexicographic) for non-numeric columns
        match &str_min {
            Some(m) if *m <= s => {}
            _ => str_min = Some(s.clone()),
        }
        match &str_max {
            Some(m) if *m >= s => {}
            _ => str_max = Some(s.clone()),
        }

        if samples.len() < SAMPLES {
            samples.push(s.clone());
        }
        if !freq_full {
            *freq.entry(s).or_insert(0) += 1;
            if freq.len() >= DISTINCT_CAP {
                freq_full = true;
            }
        }
    }

    let inferred = if !seen_any {
        InferredType::Empty
    } else if is_bool {
        InferredType::Boolean
    } else if is_int {
        InferredType::Integer
    } else if is_float {
        InferredType::Float
    } else {
        InferredType::String
    };

    let numeric = matches!(inferred, InferredType::Integer | InferredType::Float);
    let (min, max, mean) = if numeric && num_n > 0 {
        (
            Some(trim_float(num_min)),
            Some(trim_float(num_max)),
            Some(num_sum / num_n as f64),
        )
    } else {
        (str_min, str_max, None)
    };

    let mut top: Vec<ValueCount> = freq
        .into_iter()
        .map(|(value, count)| ValueCount { value, count })
        .collect();
    top.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.value.cmp(&b.value)));
    let distinct = top.len() as u64;
    top.truncate(TOP_N);

    ColumnProfile {
        column: name,
        index: col,
        inferred_type: inferred,
        count,
        nulls,
        blanks,
        distinct,
        distinct_capped: freq_full,
        min,
        max,
        mean,
        top,
        samples,
    }
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn trim_float(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{}", f as i64)
    } else {
        format!("{f}")
    }
}
