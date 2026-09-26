//! Cover-type detection: Method A (LSB) vs Method B (EOF) — stage 6.

use crate::error::{StegoError, StegoResult};

/// Embedding strategy selected for a cover file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingMethod {
    /// Keyed random LSB for PNG/BMP covers.
    Lsb,
    /// Keyed EOF append for generic covers.
    Eof,
}

/// Detect which method to use from a file path / extension (stub).
pub fn detect_method(_path: &str) -> StegoResult<EmbeddingMethod> {
    Err(StegoError::NotImplemented("detection::detect_method"))
}
