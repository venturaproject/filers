//! Operations that run on a parsed file or on JSON tabular data: profiling,
//! schema validation, format conversion, declarative transformation, diffing,
//! and `.xlsx` generation.

pub mod convert;
pub mod diff;
pub mod generate;
pub mod pipeline;
pub mod profile;
pub mod transform;
pub mod validate;
