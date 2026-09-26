//! Method B — keyed EOF append for generic covers.
//!
//! Layout (parse from the end of the file):
//! ```text
//! [cover bytes][encrypted envelope][salt 16][hmac 32][envelope_len u64 LE]
//! ```
//!
//! `hmac = HMAC-SHA256(LocatorKey, envelope)` where
//! `LocatorKey` comes from Argon2id(password, salt) as in [`crate::crypto`].
//! There is no fixed plaintext magic marker in the clear.

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::crypto::{self, SALT_LEN};
use crate::error::{StegoError, StegoResult};

type HmacSha256 = Hmac<Sha256>;

const FOOTER_LEN: usize = SALT_LEN + 32 + 8;

/// Append encrypted envelope + keyed locator footer after cover bytes.
pub fn embed(cover: &[u8], ciphertext: &[u8], password: &str) -> StegoResult<Vec<u8>> {
    if ciphertext.len() < 1 + SALT_LEN {
        return Err(StegoError::InvalidFormat("ciphertext too short".into()));
    }
    // Envelope already contains its own salt at bytes [1..17]; reuse for footer
    // so extract can derive LocatorKey before decrypting.
    let salt = &ciphertext[1..1 + SALT_LEN];
    let keys = crypto::derive_keys(password, salt)?;

    let mut mac = HmacSha256::new_from_slice(&keys.locator_key)
        .map_err(|e| StegoError::Message(format!("hmac key: {e}")))?;
    mac.update(ciphertext);
    let tag = mac.finalize().into_bytes();

    let mut out = Vec::with_capacity(cover.len() + ciphertext.len() + FOOTER_LEN);
    out.extend_from_slice(cover);
    out.extend_from_slice(ciphertext);
    out.extend_from_slice(salt);
    out.extend_from_slice(&tag);
    out.extend_from_slice(&(ciphertext.len() as u64).to_le_bytes());
    Ok(out)
}

/// Locate and return the encrypted envelope from a Method B stego file.
pub fn extract(stego: &[u8], password: &str) -> StegoResult<Vec<u8>> {
    if stego.len() < FOOTER_LEN + 1 {
        return Err(StegoError::InvalidFormat(
            "file too short for EOF footer".into(),
        ));
    }

    let len_bytes: [u8; 8] = stego[stego.len() - 8..].try_into().unwrap();
    let env_len = u64::from_le_bytes(len_bytes) as usize;
    if env_len == 0 || stego.len() < FOOTER_LEN + env_len {
        return Err(StegoError::WrongPassword);
    }

    let footer_start = stego.len() - FOOTER_LEN;
    let env_start = footer_start - env_len;
    let envelope = &stego[env_start..footer_start];
    let salt = &stego[footer_start..footer_start + SALT_LEN];
    let tag = &stego[footer_start + SALT_LEN..footer_start + SALT_LEN + 32];

    // Envelope salt must match footer salt (same encryption).
    if envelope.len() < 1 + SALT_LEN || &envelope[1..1 + SALT_LEN] != salt {
        return Err(StegoError::WrongPassword);
    }

    let keys = crypto::derive_keys(password, salt)?;
    let mut mac = HmacSha256::new_from_slice(&keys.locator_key)
        .map_err(|e| StegoError::Message(format!("hmac key: {e}")))?;
    mac.update(envelope);
    if mac.verify_slice(tag).is_err() {
        return Err(StegoError::WrongPassword);
    }

    Ok(envelope.to_vec())
}

/// Original cover length if footer parses (does not verify password).
pub fn cover_prefix_len(stego: &[u8]) -> StegoResult<usize> {
    if stego.len() < FOOTER_LEN {
        return Err(StegoError::InvalidFormat("too short".into()));
    }
    let env_len = u64::from_le_bytes(stego[stego.len() - 8..].try_into().unwrap()) as usize;
    if stego.len() < FOOTER_LEN + env_len {
        return Err(StegoError::InvalidFormat("invalid envelope length".into()));
    }
    Ok(stego.len() - FOOTER_LEN - env_len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::encrypt_blob;

    fn sample_cover(kind: &str) -> Vec<u8> {
        match kind {
            "pdf" => {
                let mut v = b"%PDF-1.4\n1 0 obj<<>>endobj\ntrailer<<>>\n%%EOF\n".to_vec();
                v.extend_from_slice(b"more pdf-like bytes");
                v
            }
            "mp3" => {
                // Minimal ID3-ish + junk (not a real frame; fine for EOF tests)
                let mut v = b"ID3".to_vec();
                v.extend_from_slice(&[0u8; 64]);
                v.extend_from_slice(b"mp3-audio-placeholder");
                v
            }
            "zip" => {
                // Local file header signature + padding + EOCD-like end
                let mut v = vec![0x50, 0x4b, 0x03, 0x04];
                v.extend_from_slice(&[0u8; 26]);
                v.extend_from_slice(b"zip-payload");
                v.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
                v.extend_from_slice(&[0u8; 18]);
                v
            }
            _ => b"generic-cover".to_vec(),
        }
    }

    #[test]
    fn round_trip_pdf_like() {
        let cover = sample_cover("pdf");
        let pw = "eof-secret";
        let ct = encrypt_blob(b"hidden-in-pdf", pw).unwrap();
        let stego = embed(&cover, &ct, pw).unwrap();
        assert!(stego.len() > cover.len());
        let extracted = extract(&stego, pw).unwrap();
        assert_eq!(extracted, ct);
        assert_eq!(
            crate::crypto::decrypt_blob(&extracted, pw).unwrap(),
            b"hidden-in-pdf"
        );
    }

    #[test]
    fn round_trip_mp3_and_zip_like() {
        let pw = "media-pass";
        for kind in ["mp3", "zip"] {
            let cover = sample_cover(kind);
            let ct = encrypt_blob(format!("payload-{kind}").as_bytes(), pw).unwrap();
            let stego = embed(&cover, &ct, pw).unwrap();
            let got = extract(&stego, pw).unwrap();
            assert_eq!(got, ct, "kind={kind}");
        }
    }

    #[test]
    fn wrong_password_fails() {
        let cover = sample_cover("pdf");
        let ct = encrypt_blob(b"x", "right").unwrap();
        let stego = embed(&cover, &ct, "right").unwrap();
        assert_eq!(
            extract(&stego, "wrong").unwrap_err(),
            StegoError::WrongPassword
        );
    }

    #[test]
    fn no_footer_fails() {
        let err = extract(b"%PDF bare file", "pw").unwrap_err();
        match err {
            StegoError::InvalidFormat(_) | StegoError::WrongPassword => {}
            other => panic!("{other:?}"),
        }
    }
}
