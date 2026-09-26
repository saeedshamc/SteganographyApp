//! Argon2id key derivation and AES-256-GCM encryption.
//!
//! Envelope layouts:
//! ```text
//! v1 (Balanced, backward compatible):
//!   [version=1][salt: 16][nonce: 12][ciphertext || 16-byte GCM tag]
//!
//! v2 (explicit KDF profile):
//!   [version=2][profile: u8][salt: 16][nonce: 12][ciphertext || tag]
//! ```
//!
//! From Argon2id(password_material, salt) we derive 64 bytes, then split via HKDF-SHA256 into:
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
pub const FORMAT_VERSION_V2: u8 = 2;

/// Argon2id cost profile (educational Fast / default Balanced / Paranoid).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KdfProfile {
    /// Lower memory/time — faster demos, weaker against offline attacks.
    Fast = 1,
    /// Default (~19 MiB, t=2) — matches historical v1 envelopes.
    #[default]
    Balanced = 2,
    /// Higher cost — slower, stronger.
    Paranoid = 3,
}

impl KdfProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            KdfProfile::Fast => "fast",
            KdfProfile::Balanced => "balanced",
            KdfProfile::Paranoid => "paranoid",
        }
    }

    pub fn from_u8(v: u8) -> StegoResult<Self> {
        match v {
            1 => Ok(KdfProfile::Fast),
            2 => Ok(KdfProfile::Balanced),
            3 => Ok(KdfProfile::Paranoid),
            _ => Err(StegoError::InvalidFormat(format!(
                "unknown kdf profile id {v}"
            ))),
        }
    }

    pub fn parse(s: &str) -> StegoResult<Self> {
        match s.to_ascii_lowercase().as_str() {
            "fast" => Ok(KdfProfile::Fast),
            "balanced" | "default" => Ok(KdfProfile::Balanced),
            "paranoid" => Ok(KdfProfile::Paranoid),
            other => Err(StegoError::Message(format!(
                "unknown kdf profile '{other}' (fast|balanced|paranoid)"
            ))),
        }
    }

    fn argon_params(self) -> (u32, u32, u32) {
        // (m_kib, t_cost, p_cost)
        match self {
            KdfProfile::Fast => (8_192, 1, 1),
            KdfProfile::Balanced => (19_456, 2, 1),
            KdfProfile::Paranoid => (65_536, 3, 1),
        }
    }

    pub fn all() -> [KdfProfile; 3] {
        [KdfProfile::Balanced, KdfProfile::Fast, KdfProfile::Paranoid]
    }
}

/// Optional knobs for encryption / key derivation.
#[derive(Debug, Clone, Default)]
pub struct CryptoOptions {
    pub profile: KdfProfile,
    /// Optional keyfile bytes mixed into Argon2 password input.
    pub keyfile: Option<Vec<u8>>,
}

/// Derived key material (zeroized on drop).
#[derive(Zeroize, ZeroizeOnDrop)]
pub struct DerivedKeys {
    pub enc_key: [u8; KEY_LEN],
    pub locator_key: [u8; KEY_LEN],
}

fn password_material(password: &str, keyfile: Option<&[u8]>) -> Vec<u8> {
    match keyfile {
        None | Some([]) => password.as_bytes().to_vec(),
        Some(kf) => {
            let mut v = Vec::with_capacity(password.len() + 1 + kf.len());
            v.extend_from_slice(password.as_bytes());
            v.push(0xff);
            v.extend_from_slice(kf);
            v
        }
    }
}

fn argon2_for(profile: KdfProfile) -> StegoResult<Argon2<'static>> {
    let (m, t, p) = profile.argon_params();
    let params = Params::new(m, t, p, Some(64))
        .map_err(|e| StegoError::Message(format!("argon2 params: {e}")))?;
    Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
}

/// Derive EncKey + LocatorKey from password (+ optional keyfile) and salt.
pub fn derive_keys(password: &str, salt: &[u8]) -> StegoResult<DerivedKeys> {
    derive_keys_with(password, salt, &CryptoOptions::default())
}

pub fn derive_keys_with(
    password: &str,
    salt: &[u8],
    opts: &CryptoOptions,
) -> StegoResult<DerivedKeys> {
    if salt.len() != SALT_LEN {
        return Err(StegoError::InvalidFormat(format!(
            "salt must be {SALT_LEN} bytes"
        )));
    }

    let material = password_material(password, opts.keyfile.as_deref());
    let mut master = [0u8; 64];
    argon2_for(opts.profile)?
        .hash_password_into(&material, salt, &mut master)
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

/// Parsed cleartext header of an encrypted envelope.
#[derive(Debug, Clone)]
pub struct EnvelopeHeader {
    pub version: u8,
    pub profile: KdfProfile,
    pub salt: [u8; SALT_LEN],
    pub nonce: [u8; NONCE_LEN],
    pub ciphertext_offset: usize,
}

pub fn parse_envelope_header(envelope: &[u8]) -> StegoResult<EnvelopeHeader> {
    if envelope.is_empty() {
        return Err(StegoError::InvalidFormat("encrypted blob empty".into()));
    }
    match envelope[0] {
        FORMAT_VERSION => {
            let need = 1 + SALT_LEN + NONCE_LEN + 16;
            if envelope.len() < need {
                return Err(StegoError::InvalidFormat(
                    "encrypted blob too short".into(),
                ));
            }
            let mut salt = [0u8; SALT_LEN];
            salt.copy_from_slice(&envelope[1..1 + SALT_LEN]);
            let mut nonce = [0u8; NONCE_LEN];
            nonce.copy_from_slice(&envelope[1 + SALT_LEN..1 + SALT_LEN + NONCE_LEN]);
            Ok(EnvelopeHeader {
                version: FORMAT_VERSION,
                profile: KdfProfile::Balanced,
                salt,
                nonce,
                ciphertext_offset: 1 + SALT_LEN + NONCE_LEN,
            })
        }
        FORMAT_VERSION_V2 => {
            let need = 1 + 1 + SALT_LEN + NONCE_LEN + 16;
            if envelope.len() < need {
                return Err(StegoError::InvalidFormat(
                    "encrypted blob too short (v2)".into(),
                ));
            }
            let profile = KdfProfile::from_u8(envelope[1])?;
            let mut salt = [0u8; SALT_LEN];
            salt.copy_from_slice(&envelope[2..2 + SALT_LEN]);
            let mut nonce = [0u8; NONCE_LEN];
            nonce.copy_from_slice(&envelope[2 + SALT_LEN..2 + SALT_LEN + NONCE_LEN]);
            Ok(EnvelopeHeader {
                version: FORMAT_VERSION_V2,
                profile,
                salt,
                nonce,
                ciphertext_offset: 2 + SALT_LEN + NONCE_LEN,
            })
        }
        v => Err(StegoError::InvalidFormat(format!(
            "unsupported format version {v}"
        ))),
    }
}

/// Encrypt plaintext with default Balanced profile (v1 envelope).
pub fn encrypt_blob(plaintext: &[u8], password: &str) -> StegoResult<Vec<u8>> {
    encrypt_blob_with(plaintext, password, &CryptoOptions::default())
}

pub fn encrypt_blob_with(
    plaintext: &[u8],
    password: &str,
    opts: &CryptoOptions,
) -> StegoResult<Vec<u8>> {
    let salt = random_bytes::<SALT_LEN>();
    let nonce_bytes = random_bytes::<NONCE_LEN>();
    let keys = derive_keys_with(password, &salt, opts)?;

    let cipher = Aes256Gcm::new_from_slice(&keys.enc_key)
        .map_err(|e| StegoError::Message(format!("aes key: {e}")))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| StegoError::Message("encryption failed".into()))?;

    // Balanced + no keyfile keeps v1 for compatibility; otherwise v2.
    let use_v2 = opts.profile != KdfProfile::Balanced || opts.keyfile.as_ref().is_some_and(|k| !k.is_empty());
    let mut out = Vec::with_capacity(2 + SALT_LEN + NONCE_LEN + ciphertext.len());
    if use_v2 {
        out.push(FORMAT_VERSION_V2);
        out.push(opts.profile as u8);
    } else {
        out.push(FORMAT_VERSION);
    }
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypt an envelope produced by [`encrypt_blob`] / [`encrypt_blob_with`].
pub fn decrypt_blob(envelope: &[u8], password: &str) -> StegoResult<Vec<u8>> {
    decrypt_blob_with(envelope, password, None)
}

/// Decrypt; `keyfile` must match hide-time keyfile if one was used.
pub fn decrypt_blob_with(
    envelope: &[u8],
    password: &str,
    keyfile: Option<&[u8]>,
) -> StegoResult<Vec<u8>> {
    let header = parse_envelope_header(envelope)?;
    let opts = CryptoOptions {
        profile: header.profile,
        keyfile: keyfile.map(|k| k.to_vec()),
    };
    let keys = derive_keys_with(password, &header.salt, &opts)?;
    let cipher = Aes256Gcm::new_from_slice(&keys.enc_key)
        .map_err(|e| StegoError::Message(format!("aes key: {e}")))?;
    let nonce = Nonce::from_slice(&header.nonce);
    let ciphertext = &envelope[header.ciphertext_offset..];

    cipher.decrypt(nonce, ciphertext).map_err(|_| StegoError::WrongPassword)
}

/// Salt bytes stored in Method A sequential LSBs / Method B footer mirror.
pub fn salt_from_envelope(envelope: &[u8]) -> StegoResult<[u8; SALT_LEN]> {
    Ok(parse_envelope_header(envelope)?.salt)
}

/// Extract locator key for Method A/B.
pub fn locator_key_from_envelope(envelope: &[u8], password: &str) -> StegoResult<[u8; KEY_LEN]> {
    locator_key_from_envelope_with(envelope, password, None)
}

pub fn locator_key_from_envelope_with(
    envelope: &[u8],
    password: &str,
    keyfile: Option<&[u8]>,
) -> StegoResult<[u8; KEY_LEN]> {
    let header = parse_envelope_header(envelope)?;
    let opts = CryptoOptions {
        profile: header.profile,
        keyfile: keyfile.map(|k| k.to_vec()),
    };
    Ok(derive_keys_with(password, &header.salt, &opts)?.locator_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_encrypt_decrypt() {
        let msg = b"hello open stego secret payload";
        let password = "correct horse battery staple";
        let enc = encrypt_blob(msg, password).expect("encrypt");
        assert_eq!(enc[0], FORMAT_VERSION);
        let dec = decrypt_blob(&enc, password).expect("decrypt");
        assert_eq!(dec, msg);
    }

    #[test]
    fn round_trip_paranoid_v2() {
        let opts = CryptoOptions {
            profile: KdfProfile::Paranoid,
            keyfile: None,
        };
        let enc = encrypt_blob_with(b"x", "pw", &opts).unwrap();
        assert_eq!(enc[0], FORMAT_VERSION_V2);
        assert_eq!(enc[1], KdfProfile::Paranoid as u8);
        assert_eq!(decrypt_blob(&enc, "pw").unwrap(), b"x");
    }

    #[test]
    fn round_trip_with_keyfile() {
        let opts = CryptoOptions {
            profile: KdfProfile::Fast,
            keyfile: Some(b"keyfile-bytes".to_vec()),
        };
        let enc = encrypt_blob_with(b"payload", "pw", &opts).unwrap();
        assert_eq!(
            decrypt_blob_with(&enc, "pw", Some(b"keyfile-bytes")).unwrap(),
            b"payload"
        );
        assert_eq!(
            decrypt_blob_with(&enc, "pw", None).unwrap_err(),
            StegoError::WrongPassword
        );
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
    fn profiles_differ() {
        let salt = [9u8; SALT_LEN];
        let a = derive_keys_with(
            "pw",
            &salt,
            &CryptoOptions {
                profile: KdfProfile::Fast,
                keyfile: None,
            },
        )
        .unwrap();
        let b = derive_keys_with(
            "pw",
            &salt,
            &CryptoOptions {
                profile: KdfProfile::Balanced,
                keyfile: None,
            },
        )
        .unwrap();
        assert_ne!(a.enc_key, b.enc_key);
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
