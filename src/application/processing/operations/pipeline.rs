//! Run an ordered list of operations on one uploaded file in a single request.
//! `validate` → `transform` → `profile` in any order, optionally ending in a
//! terminal `convert` that produces a downloadable file.

use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{convert, generate, profile, transform, validate};
use crate::domain::processing::entities::ParsedFile;
use crate::errors::{AppError, AppResult};

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum Step {
    Validate {
        schema: validate::Schema,
        /// Abort the pipeline with 422 when the file is invalid. Default `true`.
        #[serde(default = "default_true")]
        fail_on_error: bool,
    },
    Transform {
        spec: transform::Spec,
    },
    Profile,
    Convert {
        to: String,
        #[serde(default)]
        out_delimiter: Option<char>,
        #[serde(default)]
        xlsx: generate::XlsxOptions,
    },
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
pub struct Pipeline {
    pub steps: Vec<Step>,
}

#[derive(Debug, Serialize)]
pub struct StepReport {
    pub op: &'static str,
    pub ms: u128,
    #[serde(flatten)]
    pub detail: Value,
}

pub enum Outcome {
    /// The pipeline ended without a terminal step — the working data + reports.
    Data {
        file: Box<ParsedFile>,
        steps: Vec<StepReport>,
    },
    /// The pipeline ended in `convert` — the rendered file + its target.
    File {
        bytes: Vec<u8>,
        target: convert::Target,
        steps: Vec<StepReport>,
    },
    /// A `validate` step with `fail_on_error` failed — the handler returns 422.
    Rejected { steps: Vec<StepReport> },
}

pub fn run(initial: ParsedFile, pipeline: &Pipeline) -> AppResult<Outcome> {
    let mut file = initial;
    let mut steps: Vec<StepReport> = Vec::with_capacity(pipeline.steps.len());
    let last = pipeline.steps.len().saturating_sub(1);

    for (i, step) in pipeline.steps.iter().enumerate() {
        let t = Instant::now();
        match step {
            Step::Validate {
                schema,
                fail_on_error,
            } => {
                let report = validate::run(&file, schema)?;
                let valid = report.valid;
                let detail = serde_json::to_value(&report).unwrap_or(Value::Null);
                steps.push(StepReport {
                    op: "validate",
                    ms: t.elapsed().as_millis(),
                    detail,
                });
                if *fail_on_error && !valid {
                    return Ok(Outcome::Rejected { steps });
                }
            }
            Step::Transform { spec } => {
                let out = transform::run(&file, spec, t)?;
                steps.push(StepReport {
                    op: "transform",
                    ms: t.elapsed().as_millis(),
                    detail: json!({
                        "matched_rows": out.matched_rows,
                        "returned_rows": out.file.stats.returned_rows,
                        "columns": out.file.stats.columns,
                    }),
                });
                file = out.file;
            }
            Step::Profile => {
                let report = profile::run(&file);
                steps.push(StepReport {
                    op: "profile",
                    ms: t.elapsed().as_millis(),
                    detail: serde_json::to_value(&report).unwrap_or(Value::Null),
                });
            }
            Step::Convert {
                to,
                out_delimiter,
                xlsx,
            } => {
                if i != last {
                    return Err(AppError::BadRequest(
                        "`convert` must be the last pipeline step".into(),
                    ));
                }
                let target = convert::Target::parse(to).ok_or_else(|| {
                    AppError::BadRequest("`to` must be csv, json, ndjson or xlsx".into())
                })?;
                let bytes = match target {
                    convert::Target::Xlsx => generate::from_parsed(&file, xlsx)?,
                    _ => convert::run(&file, target, out_delimiter.unwrap_or(',') as u8)?,
                };
                steps.push(StepReport {
                    op: "convert",
                    ms: t.elapsed().as_millis(),
                    detail: json!({ "to": target.extension(), "bytes": bytes.len() }),
                });
                return Ok(Outcome::File {
                    bytes,
                    target,
                    steps,
                });
            }
        }
    }

    Ok(Outcome::Data {
        file: Box::new(file),
        steps,
    })
}
