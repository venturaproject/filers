//! Operations that run on a parsed file: profiling, schema validation, format
//! conversion, declarative transformation, and diffing two files.

pub mod convert;
pub mod diff;
pub mod profile;
pub mod transform;
pub mod validate;
