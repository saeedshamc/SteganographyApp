//! Argon2id key derivation and AES-256-GCM (implemented in stage 2).

use crate::error::{StegoError, StegoResult};

/// Derive encryption and locator keys from a password (stub).
pub fn derive_keys(_password: &str, _salt: &[u8]) -> StegoResult<(Vec<u8>, Vec<u8>)> {
    Err(StegoError::NotImplemented("crypto::derive_keys"))
}

/// Encrypt an arbitrary blob (stub).
pub fn encrypt_blob(_plaintext: &[u8], _password: &str) -> StegoResult<Vec<u8>> {
    Err(StegoError::NotImplemented("crypto::encrypt_blob"))
}

/// Decrypt a blob produced by [`encrypt_blob`] (stub).
pub fn decrypt_blob(_ciphertext: &[u8], _password: &str) -> StegoResult<Vec<u8>> {
    Err(StegoError::NotImplemented("crypto::decrypt_blob"))
}
