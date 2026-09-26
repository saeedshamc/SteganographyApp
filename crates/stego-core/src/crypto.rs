//! Argon2id key derivation and AES-256-GCM encryption.
//!
//! On-disk / in-image encrypted envelope:
//! ```text
//! [version: u8 = 1][salt: 16][nonce: 12][ciphertext || 16-byte GCM tag]
//! ```
//!
//! From Argon2id(password, salt) we derive 64 bytes, then split via HKDF-SHA256 into:
//! - EncKey (32 bytes) — AES-256-GCM
//! - LocatorKey (32 bytes) — Method B HMAC locator / Method A PRNG seed material

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;
use zeroize::{Zeroize, ZeroizeOnDrop};

use crate::container::FORMAT_VERSION;
use crate::error::{StegoError, StegoResult};

pub const SALT_LEN: usize = 16;
pub const NONCE_LEN: usize = 12;
pub const KEY_LEN: usize = 32;
const HEADER_LEN: usize = 1 + SALT_LEN + NONCE_LEN;

/// Argon2id memory cost in KiB (19 MiB — OWASP-ish, still practical on desktop).
const ARGON2_M_KIB: u32 = 19_456;
const ARGON2_T_COST: u32 = 2;
const ARGON2_P_COST: u32 = 1;

/// Derived key material (zeroized on drop).
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct DerivedKeys {
    pub enc_key: [u8; KEY_LEN],
    pub locator_key: [u8; KEY_LEN],
}

fn argon2() -> StegoResult<Argon2<'static>> {
    let params = Params::new(ARGON2_M_KIB, ARGON2_T_COST, ARGON2_P_COST, Some(64))
        .map_err(|e| StegoError::Message(format!("argon2 params: {e}")))?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

/// Derive EncKey + LocatorKey from password and salt.
pub fn derive_keys(password: &str, salt: &[u8]) -> StegoResult<DerivedKeys> {
    if salt.len() != SALT_LEN {
        return Err(StegoError::InvalidFormat(format!(
            "salt must be {SALT_LEN} bytes"
        )));
    }

    let mut master = [0u8; 64];
    argon2()?
        .hash_password_into(password.as_bytes(), salt, &mut master)
        .map_err(|e| StegoError::Message(format!("argon2 failed: {e}")))?;

    let hk = Hkdf::<Sha256>::new(Some(salt), &master);
    master.zeroize();

    let mut enc_key = [0u8; KEY_LEN];
    let mut locator_key = [0u8; KEY_LEN];
    hk.expand(b"OpenStego/enc/v1", &mut enc_key)
        .map_err(|_| StegoError::Message("hkdf expand enc failed".into()))?;
    hk.expand(b"OpenStego/locator/v1", &mut locator_key)
        .map_err(|_| StegoError::Message("hkdf expand locator failed".into()))?;

    Ok(DerivedKeys {
        enc_key,
        locator_key,
    })
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    rand::thread_rng().fill_bytes(&mut buf);
    buf
}

/// Encrypt plaintext into the versioned envelope described above.
pub fn encrypt_blob(plaintext: &[u8], password: &str) -> StegoResult<Vec<u8>> {
    let salt = random_bytes::<SALT_LEN>();
    let nonce_bytes = random_bytes::<NONCE_LEN>();
    let keys = derive_keys(password, &salt)?;

    let cipher = Aes256Gcm::new_from_slice(&keys.enc_key)
        .map_err(|e| StegoError::Message(format!("aes key: {e}")))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| StegoError::Message("encryption failed".into()))?;

    let mut out = Vec::with_capacity(HEADER_LEN + ciphertext.len());
    out.push(FORMAT_VERSION);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypt an envelope produced by [`encrypt_blob`].
pub fn decrypt_blob(envelope: &[u8], password: &str) -> StegoResult<Vec<u8>> {
    if envelope.len() < HEADER_LEN + 16 {
        return Err(StegoError::InvalidFormat(
            "encrypted blob too short".into(),
        ));
    }
    if envelope[0] != FORMAT_VERSION {
        return Err(StegoError::InvalidFormat(format!(
            "unsupported format version {}",
            envelope[0]
        )));
    }

    let salt = &envelope[1..1 + SALT_LEN];
    let nonce_bytes = &envelope[1 + SALT_LEN..HEADER_LEN];
    let ciphertext = &envelope[HEADER_LEN..];

    let keys = derive_keys(password, salt)?;
    let cipher = Aes256Gcm::new_from_slice(&keys.enc_key)
        .map_err(|e| StegoError::Message(format!("aes key: {e}")))?;
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher.decrypt(nonce, ciphertext).map_err(|_| {
        // GCM auth failure: wrong password or tampering — same observable outcome.
        StegoError::WrongPassword
    })
}

/// Extract locator key for Method A/B (re-derives from password + salt inside envelope).
pub fn locator_key_from_envelope(envelope: &[u8], password: &str) -> StegoResult<[u8; KEY_LEN]> {
    if envelope.len() < 1 + SALT_LEN {
        return Err(StegoError::InvalidFormat("envelope too short".into()));
    }
    if envelope[0] != FORMAT_VERSION {
        return Err(StegoError::InvalidFormat(format!(
            "unsupported format version {}",
            envelope[0]
        )));
    }
    let salt = &envelope[1..1 + SALT_LEN];
    Ok(derive_keys(password, salt)?.locator_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_encrypt_decrypt() {
        let msg = b"hello open stego secret payload";
        let password = "correct horse battery staple";
        let enc = encrypt_blob(msg, password).expect("encrypt");
        let dec = decrypt_blob(&enc, password).expect("decrypt");
        assert_eq!(dec, msg);
    }

    #[test]
    fn wrong_password_fails_clearly() {
        let enc = encrypt_blob(b"secret", "right-password").unwrap();
        let err = decrypt_blob(&enc, "wrong-password").unwrap_err();
        assert_eq!(err, StegoError::WrongPassword);
    }

    #[test]
    fn tampered_ciphertext_fails() {
        let mut enc = encrypt_blob(b"secret", "pw").unwrap();
        let last = enc.len() - 1;
        enc[last] ^= 0xff;
        let err = decrypt_blob(&enc, "pw").unwrap_err();
        assert_eq!(err, StegoError::WrongPassword);
    }

    #[test]
    fn derive_keys_deterministic() {
        let salt = [7u8; SALT_LEN];
        let a = derive_keys("pw", &salt).unwrap();
        let b = derive_keys("pw", &salt).unwrap();
        assert_eq!(a.enc_key, b.enc_key);
        assert_eq!(a.locator_key, b.locator_key);
        assert_ne!(a.enc_key, a.locator_key);
    }

    #[test]
    fn short_blob_invalid_format() {
        let err = decrypt_blob(&[1, 2, 3], "pw").unwrap_err();
        match err {
            StegoError::InvalidFormat(_) => {}
            other => panic!("unexpected {other:?}"),
        }
    }
}
