//! High-level hide / extract orchestration used by CLI and GUI.

use image::GenericImageView;

use crate::crypto::{
    decrypt_blob_with, encrypt_blob_with, CryptoOptions, KdfProfile,
};
use crate::detection::{detect_from_bytes, CoverDecision, EmbeddingMethod};
use crate::embedding::{method_a_lsb, method_b_eof};
use crate::error::{StegoError, StegoResult};
use crate::metadata::{unwrap_payload, wrap_payload, PayloadMeta};

/// Plan shown to the user before hiding.
#[derive(Debug, Clone)]
pub struct HidePlan {
    pub method: EmbeddingMethod,
    pub capacity: Option<usize>,
    pub jpeg_warning: Option<String>,
    pub eof_caveat: Option<String>,
}

/// Options for hide/extract (KDF profile, keyfile, LSB adaptive).
#[derive(Debug, Clone, Default)]
pub struct StegoOptions {
    pub crypto: CryptoOptions,
    /// Prefer high-variance pixels for Method A (educational; same capacity formula).
    pub adaptive_lsb: bool,
}

const EOF_CAVEAT: &str =
    "Method B appends data after the file end. Most formats ignore trailing bytes, but a few with strict validation may reject the output.";

/// Inspect cover bytes and path; return which method and capacity (if LSB).
pub fn plan_hide(cover: &[u8], cover_path: &str) -> StegoResult<HidePlan> {
    let decision = detect_from_bytes(cover, Some(cover_path))?;
    match decision {
        CoverDecision::UseLsb { .. } => {
            let img = image::load_from_memory(cover)
                .map_err(|e| StegoError::UnsupportedCover(format!("cannot decode image: {e}")))?;
            let (w, h) = img.dimensions();
            let capacity = method_a_lsb::capacity_bytes(w, h)?;
            Ok(HidePlan {
                method: EmbeddingMethod::Lsb,
                capacity: Some(capacity),
                jpeg_warning: None,
                eof_caveat: None,
            })
        }
        CoverDecision::UseEof { .. } => {
            let ext = std::path::Path::new(cover_path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let caveat = if ext == "pdf" || cover.starts_with(b"%PDF") {
                "PDF (Method B): encrypted payload is appended after the file. Most viewers ignore trailing bytes; a trailing %%EOF may still be present earlier in the file."
            } else {
                EOF_CAVEAT
            };
            Ok(HidePlan {
                method: EmbeddingMethod::Eof,
                capacity: None,
                jpeg_warning: None,
                eof_caveat: Some(caveat.into()),
            })
        }
        CoverDecision::JpegNeedsConversion => Err(StegoError::UnsupportedCover(
            "JPEG cover detected. Lossy JPEG destroys LSB data. Convert to PNG first, or choose another cover."
                .into(),
        )),
    }
}

/// Hide with default crypto options.
pub fn hide(
    cover: &[u8],
    cover_path: &str,
    meta: &PayloadMeta,
    payload: &[u8],
    password: &str,
) -> StegoResult<(Vec<u8>, String)> {
    hide_with(
        cover,
        cover_path,
        meta,
        payload,
        password,
        &StegoOptions::default(),
    )
}

pub fn hide_with(
    cover: &[u8],
    cover_path: &str,
    meta: &PayloadMeta,
    payload: &[u8],
    password: &str,
    opts: &StegoOptions,
) -> StegoResult<(Vec<u8>, String)> {
    let plan = plan_hide(cover, cover_path)?;
    let wrapped = wrap_payload(meta, payload)?;
    let envelope = encrypt_blob_with(&wrapped, password, &opts.crypto)?;

    match plan.method {
        EmbeddingMethod::Lsb => {
            if let Some(cap) = plan.capacity {
                if envelope.len() > cap {
                    return Err(StegoError::CapacityExceeded {
                        need: envelope.len(),
                        capacity: cap,
                    });
                }
            }
            let png = if opts.adaptive_lsb {
                method_a_lsb::embed_adaptive(cover, &envelope, password, &opts.crypto)?
            } else {
                method_a_lsb::embed_with(cover, &envelope, password, &opts.crypto)?
            };
            Ok((png, "png".into()))
        }
        EmbeddingMethod::Eof => {
            let stego =
                method_b_eof::embed_with(cover, &envelope, password, &opts.crypto)?;
            let ext = std::path::Path::new(cover_path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("bin")
                .to_string();
            Ok((stego, ext))
        }
    }
}

/// Recovered payload after a successful extract.
#[derive(Debug, Clone)]
pub struct ExtractedPayload {
    pub meta: PayloadMeta,
    pub data: Vec<u8>,
    pub method: EmbeddingMethod,
}

/// Extract with default options.
pub fn extract(stego: &[u8], stego_path: &str, password: &str) -> StegoResult<ExtractedPayload> {
    extract_with(stego, stego_path, password, &StegoOptions::default())
}

pub fn extract_with(
    stego: &[u8],
    stego_path: &str,
    password: &str,
    opts: &StegoOptions,
) -> StegoResult<ExtractedPayload> {
    let decision = detect_from_bytes(stego, Some(stego_path))?;
    let keyfile = opts.crypto.keyfile.as_deref();

    let try_lsb = || -> StegoResult<ExtractedPayload> {
        let envelope = if opts.adaptive_lsb {
            method_a_lsb::extract_adaptive(stego, password, &opts.crypto)
                .or_else(|_| method_a_lsb::extract_with(stego, password, &opts.crypto))?
        } else {
            method_a_lsb::extract_with(stego, password, &opts.crypto)
                .or_else(|_| method_a_lsb::extract_adaptive(stego, password, &opts.crypto))?
        };
        let plain = decrypt_blob_with(&envelope, password, keyfile)?;
        let (meta, data) = unwrap_payload(&plain)?;
        Ok(ExtractedPayload {
            meta,
            data,
            method: EmbeddingMethod::Lsb,
        })
    };

    let try_eof = || -> StegoResult<ExtractedPayload> {
        let envelope = method_b_eof::extract_with(stego, password, &opts.crypto)?;
        let plain = decrypt_blob_with(&envelope, password, keyfile)?;
        let (meta, data) = unwrap_payload(&plain)?;
        Ok(ExtractedPayload {
            meta,
            data,
            method: EmbeddingMethod::Eof,
        })
    };

    match decision {
        CoverDecision::UseLsb { .. } => try_lsb().or_else(|_| try_eof()),
        CoverDecision::UseEof { .. } | CoverDecision::JpegNeedsConversion => {
            try_eof().or_else(|_| try_lsb())
        }
    }
}

/// Parse CLI/GUI profile name.
pub fn parse_kdf_profile(s: &str) -> StegoResult<KdfProfile> {
    KdfProfile::parse(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    fn png_cover(w: u32, h: u32) -> Vec<u8> {
        let img: image::RgbaImage =
            ImageBuffer::from_fn(w, h, |x, y| Rgba([(x % 255) as u8, (y % 255) as u8, 90, 255]));
        let mut buf = Vec::new();
        image::DynamicImage::ImageRgba8(img)
            .write_to(
                &mut std::io::Cursor::new(&mut buf),
                image::ImageFormat::Png,
            )
            .unwrap();
        buf
    }

    #[test]
    fn hide_extract_png_text() {
        let cover = png_cover(160, 160);
        let data = b"hello from pipeline";
        let meta = PayloadMeta::for_text(data);
        let (stego, ext) = hide(&cover, "c.png", &meta, data, "pipe-pw").unwrap();
        assert_eq!(ext, "png");
        let out = extract(&stego, "out.png", "pipe-pw").unwrap();
        assert!(out.meta.is_text);
        assert_eq!(out.data, data);
        assert_eq!(out.method, EmbeddingMethod::Lsb);
    }

    #[test]
    fn hide_extract_pdf_file() {
        let mut cover = b"%PDF-1.4\n%%EOF\n".to_vec();
        cover.extend_from_slice(&[1u8; 64]);
        let data = b"secret-bytes";
        let meta = PayloadMeta::for_file("secret.bin", data);
        let (stego, ext) = hide(&cover, "doc.pdf", &meta, data, "eof-pw").unwrap();
        assert_eq!(ext, "pdf");
        let out = extract(&stego, "doc.pdf", "eof-pw").unwrap();
        assert_eq!(out.meta.filename.as_deref(), Some("secret.bin"));
        assert_eq!(out.data, data);
        assert_eq!(out.method, EmbeddingMethod::Eof);
    }

    #[test]
    fn hide_extract_with_keyfile_and_fast() {
        let cover = png_cover(180, 180);
        let data = b"kf";
        let meta = PayloadMeta::for_text(data);
        let opts = StegoOptions {
            crypto: CryptoOptions {
                profile: KdfProfile::Fast,
                keyfile: Some(b"kf-bytes".to_vec()),
            },
            adaptive_lsb: false,
        };
        let (stego, _) =
            hide_with(&cover, "c.png", &meta, data, "pw", &opts).unwrap();
        let out = extract_with(&stego, "c.png", "pw", &opts).unwrap();
        assert_eq!(out.data, data);
        let bad = StegoOptions {
            crypto: CryptoOptions {
                profile: KdfProfile::Fast,
                keyfile: None,
            },
            ..opts.clone()
        };
        assert!(extract_with(&stego, "c.png", "pw", &bad).is_err());
    }
}
