//! Method A — keyed random LSB for PNG/BMP (stage 5).

use crate::error::{StegoError, StegoResult};

/// Maximum embeddable payload size in bytes for an image cover (stub).
pub fn capacity_bytes(_width: u32, _height: u32) -> StegoResult<usize> {
    Err(StegoError::NotImplemented("method_a_lsb::capacity_bytes"))
}

/// Embed ciphertext into PNG/BMP pixel LSBs (stub).
pub fn embed(_cover_png_or_bmp: &[u8], _ciphertext: &[u8], _password: &str) -> StegoResult<Vec<u8>> {
    Err(StegoError::NotImplemented("method_a_lsb::embed"))
}

/// Extract ciphertext from an LSB stego PNG (stub).
pub fn extract(_stego_png: &[u8], _password: &str) -> StegoResult<Vec<u8>> {
    Err(StegoError::NotImplemented("method_a_lsb::extract"))
}
