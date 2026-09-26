//! Payload metadata wrap/unwrap with SHA-256 integrity.
//!
//! Binary layout (little-endian lengths for simplicity on desktop):
//! ```text
//! magic: b"OSMP" (Open Stego Meta Payload)
//! version: u8 = 1
//! flags: u8          // bit0 = is_text
//! name_len: u16 LE
//! name: UTF-8 bytes  // empty when is_text or no name
//! ext_len: u16 LE
//! extension: UTF-8   // without leading dot; empty if none
//! size: u64 LE       // payload byte length
//! checksum: [u8; 32] // SHA-256 of payload bytes
//! payload: [u8; size]
//! ```

use sha2::{Digest, Sha256};

use crate::error::{StegoError, StegoResult};

const MAGIC: &[u8; 4] = b"OSMP";
const META_VERSION: u8 = 1;
const FLAG_IS_TEXT: u8 = 0x01;

/// Description of a payload before encryption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadMeta {
    pub is_text: bool,
    pub filename: Option<String>,
    pub extension: Option<String>,
    pub size: u64,
    pub checksum_sha256: [u8; 32],
}

impl PayloadMeta {
    /// Build metadata for a file payload (checksum computed from `data`).
    pub fn for_file(filename: impl Into<String>, data: &[u8]) -> Self {
        let filename = filename.into();
        let extension = std::path::Path::new(&filename)
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_string());
        Self {
            is_text: false,
            filename: Some(filename),
            extension,
            size: data.len() as u64,
            checksum_sha256: sha256(data),
        }
    }

    /// Build metadata for raw text/code (no filename).
    pub fn for_text(data: &[u8]) -> Self {
        Self {
            is_text: true,
            filename: None,
            extension: None,
            size: data.len() as u64,
            checksum_sha256: sha256(data),
        }
    }
}

pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

fn write_u16(buf: &mut Vec<u8>, v: u16) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn write_u64(buf: &mut Vec<u8>, v: u64) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn read_u16(data: &[u8], offset: &mut usize) -> StegoResult<u16> {
    if *offset + 2 > data.len() {
        return Err(StegoError::InvalidFormat("truncated metadata".into()));
    }
    let v = u16::from_le_bytes(data[*offset..*offset + 2].try_into().unwrap());
    *offset += 2;
    Ok(v)
}

fn read_u64(data: &[u8], offset: &mut usize) -> StegoResult<u64> {
    if *offset + 8 > data.len() {
        return Err(StegoError::InvalidFormat("truncated metadata".into()));
    }
    let v = u64::from_le_bytes(data[*offset..*offset + 8].try_into().unwrap());
    *offset += 8;
    Ok(v)
}

fn read_bytes<'a>(data: &'a [u8], offset: &mut usize, len: usize) -> StegoResult<&'a [u8]> {
    if *offset + len > data.len() {
        return Err(StegoError::InvalidFormat("truncated metadata".into()));
    }
    let slice = &data[*offset..*offset + len];
    *offset += len;
    Ok(slice)
}

/// Serialize metadata + payload into a single blob (then encrypted by crypto layer).
pub fn wrap_payload(meta: &PayloadMeta, data: &[u8]) -> StegoResult<Vec<u8>> {
    if data.len() as u64 != meta.size {
        return Err(StegoError::Message(format!(
            "meta.size {} != data.len {}",
            meta.size,
            data.len()
        )));
    }
    let checksum = sha256(data);
    if checksum != meta.checksum_sha256 {
        return Err(StegoError::Message(
            "meta checksum does not match payload bytes".into(),
        ));
    }

    let name = meta.filename.as_deref().unwrap_or("");
    let ext = meta.extension.as_deref().unwrap_or("");
    if name.len() > u16::MAX as usize || ext.len() > u16::MAX as usize {
        return Err(StegoError::Message("filename or extension too long".into()));
    }

    let mut out = Vec::with_capacity(64 + data.len());
    out.extend_from_slice(MAGIC);
    out.push(META_VERSION);
    let mut flags = 0u8;
    if meta.is_text {
        flags |= FLAG_IS_TEXT;
    }
    out.push(flags);
    write_u16(&mut out, name.len() as u16);
    out.extend_from_slice(name.as_bytes());
    write_u16(&mut out, ext.len() as u16);
    out.extend_from_slice(ext.as_bytes());
    write_u64(&mut out, meta.size);
    out.extend_from_slice(&checksum);
    out.extend_from_slice(data);
    Ok(out)
}

/// Deserialize blob and verify SHA-256 of the payload.
pub fn unwrap_payload(blob: &[u8]) -> StegoResult<(PayloadMeta, Vec<u8>)> {
    if blob.len() < 4 + 1 + 1 + 2 + 2 + 8 + 32 {
        return Err(StegoError::InvalidFormat("metadata blob too short".into()));
    }
    let mut offset = 0usize;
    let magic = read_bytes(blob, &mut offset, 4)?;
    if magic != MAGIC {
        return Err(StegoError::InvalidFormat("bad metadata magic".into()));
    }
    let version = *read_bytes(blob, &mut offset, 1)?.first().unwrap();
    if version != META_VERSION {
        return Err(StegoError::InvalidFormat(format!(
            "unsupported metadata version {version}"
        )));
    }
    let flags = *read_bytes(blob, &mut offset, 1)?.first().unwrap();
    let is_text = flags & FLAG_IS_TEXT != 0;

    let name_len = read_u16(blob, &mut offset)? as usize;
    let name_bytes = read_bytes(blob, &mut offset, name_len)?;
    let name = if name_bytes.is_empty() {
        None
    } else {
        Some(
            String::from_utf8(name_bytes.to_vec())
                .map_err(|_| StegoError::InvalidFormat("filename not utf-8".into()))?,
        )
    };

    let ext_len = read_u16(blob, &mut offset)? as usize;
    let ext_bytes = read_bytes(blob, &mut offset, ext_len)?;
    let extension = if ext_bytes.is_empty() {
        None
    } else {
        Some(
            String::from_utf8(ext_bytes.to_vec())
                .map_err(|_| StegoError::InvalidFormat("extension not utf-8".into()))?,
        )
    };

    let size = read_u64(blob, &mut offset)?;
    let checksum_bytes = read_bytes(blob, &mut offset, 32)?;
    let mut checksum_sha256 = [0u8; 32];
    checksum_sha256.copy_from_slice(checksum_bytes);

    let payload = read_bytes(blob, &mut offset, size as usize)?.to_vec();
    if offset != blob.len() {
        return Err(StegoError::InvalidFormat(
            "trailing bytes after payload".into(),
        ));
    }

    if sha256(&payload) != checksum_sha256 {
        return Err(StegoError::ChecksumMismatch);
    }

    Ok((
        PayloadMeta {
            is_text,
            filename: name,
            extension,
            size,
            checksum_sha256,
        },
        payload,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_file_payload() {
        let data = b"file-bytes-xyz";
        let meta = PayloadMeta::for_file("secret.txt", data);
        assert_eq!(meta.extension.as_deref(), Some("txt"));
        let wrapped = wrap_payload(&meta, data).unwrap();
        let (m2, d2) = unwrap_payload(&wrapped).unwrap();
        assert_eq!(m2, meta);
        assert_eq!(d2, data);
    }

    #[test]
    fn round_trip_text_payload() {
        let data = b"fn main() {}";
        let meta = PayloadMeta::for_text(data);
        assert!(meta.is_text);
        assert!(meta.filename.is_none());
        let wrapped = wrap_payload(&meta, data).unwrap();
        let (m2, d2) = unwrap_payload(&wrapped).unwrap();
        assert_eq!(m2, meta);
        assert_eq!(d2, data);
    }

    #[test]
    fn tampered_payload_checksum_fails() {
        let data = b"abc";
        let meta = PayloadMeta::for_text(data);
        let mut wrapped = wrap_payload(&meta, data).unwrap();
        let last = wrapped.len() - 1;
        wrapped[last] ^= 0x01;
        assert_eq!(
            unwrap_payload(&wrapped).unwrap_err(),
            StegoError::ChecksumMismatch
        );
    }

    #[test]
    fn bad_magic_rejected() {
        let err = unwrap_payload(b"XXXX").unwrap_err();
        match err {
            StegoError::InvalidFormat(_) => {}
            other => panic!("{other:?}"),
        }
    }
}
