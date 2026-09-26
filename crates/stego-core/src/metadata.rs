//! Payload metadata wrap/unwrap with SHA-256 integrity (stage 3).

use crate::error::{StegoError, StegoResult};

/// Wrapped payload ready for encryption (stub types for scaffolding).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadMeta {
    pub is_text: bool,
    pub filename: Option<String>,
    pub extension: Option<String>,
    pub size: u64,
    pub checksum_sha256: [u8; 32],
}

/// Serialize metadata + payload bytes (stub).
pub fn wrap_payload(_meta: &PayloadMeta, _data: &[u8]) -> StegoResult<Vec<u8>> {
    Err(StegoError::NotImplemented("metadata::wrap_payload"))
}

/// Deserialize and verify checksum (stub).
pub fn unwrap_payload(_blob: &[u8]) -> StegoResult<(PayloadMeta, Vec<u8>)> {
    Err(StegoError::NotImplemented("metadata::unwrap_payload"))
}
