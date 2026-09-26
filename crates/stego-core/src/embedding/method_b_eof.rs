//! Method B — keyed EOF append for generic covers (stage 4).

use crate::error::{StegoError, StegoResult};

/// Append encrypted blob + keyed locator after cover bytes (stub).
pub fn embed(_cover: &[u8], _ciphertext: &[u8], _password: &str) -> StegoResult<Vec<u8>> {
    Err(StegoError::NotImplemented("method_b_eof::embed"))
}

/// Locate and return the encrypted blob from a Method B stego file (stub).
pub fn extract(_stego: &[u8], _password: &str) -> StegoResult<Vec<u8>> {
    Err(StegoError::NotImplemented("method_b_eof::extract"))
}
