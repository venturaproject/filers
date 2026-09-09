pub mod csv;
pub mod excel;
pub mod xlsx_stream;

use std::io::Cursor;

use crate::errors::{AppError, AppResult};

/// Operator-tunable ceilings applied before a spreadsheet is materialised.
#[derive(Debug, Clone, Copy)]
pub struct ParseLimits {
    /// Max dense cell count (`rows * cols`) the parser will build.
    pub max_cells: u64,
    /// Max summed *uncompressed* size of a zip upload (xlsx/ods), checked from
    /// the central directory before anything is inflated.
    pub max_uncompressed_bytes: u64,
}

impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            max_cells: 64_000_000,
            max_uncompressed_bytes: 1024 * 1024 * 1024,
        }
    }
}

impl ParseLimits {
    pub fn from_mb(max_cells: u64, max_uncompressed_mb: u64) -> Self {
        Self {
            max_cells,
            max_uncompressed_bytes: max_uncompressed_mb.saturating_mul(1024 * 1024),
        }
    }

    /// Zip-bomb guard for xlsx/ods: read the central directory (cheap, no
    /// inflation) and reject if the members sum to more than the limit, or if a
    /// ZIP64 entry hides its real size.
    pub fn check_zip(&self, bytes: &[u8]) -> AppResult<()> {
        let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
            .map_err(|e| AppError::BadRequest(format!("not a valid zip container: {e}")))?;

        let mut total: u64 = 0;
        for i in 0..archive.len() {
            let entry = archive
                .by_index_raw(i)
                .map_err(|e| AppError::BadRequest(format!("corrupt zip entry: {e}")))?;
            total = total.saturating_add(entry.size());
        }

        if total > self.max_uncompressed_bytes {
            return Err(AppError::BadRequest(format!(
                "spreadsheet expands to {total} bytes uncompressed; the limit is {}",
                self.max_uncompressed_bytes
            )));
        }
        Ok(())
    }
}
