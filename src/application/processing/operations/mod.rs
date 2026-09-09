//! Operations that run on a parsed file: profiling, schema validation, and
//! format conversion. Each is a pure function over [`ParsedFile`].

pub mod convert;
pub mod profile;
pub mod validate;
