//! High-level hide / extract orchestration used by CLI and GUI.

use image::GenericImageView;

use crate::crypto::{
    decrypt_blob_with, encrypt_blob_with, CryptoOptions, KdfProfile,
};
use crate::detection::{detect_from_bytes, CoverDecision, EmbeddingMethod};
use crate::embedding::{format_aware, method_a_lsb, method_b_eof};
use crate::error::{StegoError, StegoResult};
use crate::metadata::{unwrap_payload, wrap_payload, PayloadMeta};

/// Plan shown to the user before hiding.
#[derive(Debug, Clone)]
pub struct HidePlan {
    pub method: EmbeddingMethod,
    pub capacity: Option<usize>,
    pub jpeg_warning: Option<String>,
    pub eof_caveat: Option<String>,
    /// Set when a known payload size would stress LSB capacity.
    pub capacity_risk: Option<String>,
}

/// Options for hide/extract (KDF profile, keyfile, LSB adaptive).
#[derive(Debug, Clone)]
pub struct StegoOptions {
    pub crypto: CryptoOptions,
    /// Prefer high-variance pixels for Method A (lower capacity).
    pub adaptive_lsb: bool,
    /// LSB depth 1 (default) or 2 (denser; noisier — educational).
    pub lsb_depth: u8,
}

impl Default for StegoOptions {
    fn default() -> Self {
        Self {
            crypto: CryptoOptions::default(),
            adaptive_lsb: false,
            lsb_depth: 1,
        }
    }
}

/// Inspect cover bytes and path; return which method and capacity (if LSB).
pub fn plan_hide(cover: &[u8], cover_path: &str) -> StegoResult<HidePlan> {
    plan_hide_with(cover, cover_path, &StegoOptions::default(), None)
}

pub fn plan_hide_with(
    cover: &[u8],
    cover_path: &str,
    opts: &StegoOptions,
    payload_len: Option<usize>,
) -> StegoResult<HidePlan> {
    let decision = detect_from_bytes(cover, Some(cover_path))?;
    match decision {
        CoverDecision::UseLsb { .. } => {
            let img = image::load_from_memory(cover)
                .map_err(|e| StegoError::UnsupportedCover(format!("cannot decode image: {e}")))?;
            let (w, h) = img.dimensions();
            let depth = format_aware::normalize_lsb_depth(opts.lsb_depth)?;
            let mut capacity = method_a_lsb::capacity_bytes_depth(w, h, depth)?;
            if opts.adaptive_lsb {
                capacity /= 2;
            }
            let capacity_risk = payload_len.and_then(|n| {
                // Envelope is larger than raw payload; approximate with +96 bytes overhead.
                format_aware::capacity_risk_note(n.saturating_add(96), capacity)
            });
            let mut depth_warning = None;
            if depth == 2 {
                depth_warning = Some(
                    "LSB depth 2 increases capacity but also visible noise risk — prefer depth 1 for stealth demos."
                        .into(),
                );
            }
            Ok(HidePlan {
                method: EmbeddingMethod::Lsb,
                capacity: Some(capacity),
                jpeg_warning: depth_warning,
                eof_caveat: None,
                capacity_risk,
            })
        }
        CoverDecision::UseEof { .. } => Ok(HidePlan {
            method: EmbeddingMethod::Eof,
            capacity: None,
            jpeg_warning: None,
            eof_caveat: Some(format_aware::format_caveat(cover, cover_path)),
            capacity_risk: None,
        }),
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
    let _ = format_aware::normalize_lsb_depth(opts.lsb_depth)?;
    let plan = plan_hide_with(cover, cover_path, opts, Some(payload.len()))?;
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
            let png = method_a_lsb::embed_ex(
                cover,
                &envelope,
                password,
                &opts.crypto,
                opts.lsb_depth,
                opts.adaptive_lsb,
            )?;
            Ok((png, "png".into()))
        }
        EmbeddingMethod::Eof => {
            let prepared = format_aware::prepare_cover(cover, cover_path);
            let stego =
                method_b_eof::embed_with(&prepared, &envelope, password, &opts.crypto)?;
            let stego = format_aware::finalize_stego(stego, cover, cover_path);
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
        let envelope = method_a_lsb::extract_ex(
            stego,
            password,
            &opts.crypto,
            opts.lsb_depth,
            opts.adaptive_lsb,
        )?;
        let plain = decrypt_blob_with(&envelope, password, keyfile)?;
        let (meta, data) = unwrap_payload(&plain)?;
        Ok(ExtractedPayload {
            meta,
            data,
            method: EmbeddingMethod::Lsb,
        })
    };

    let try_eof = || -> StegoResult<ExtractedPayload> {
        let body = format_aware::strip_for_extract(stego, stego_path);
        let envelope = method_b_eof::extract_with(&body, password, &opts.crypto)
            .or_else(|_| method_b_eof::extract_with(stego, password, &opts.crypto))?;
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
    fn hide_extract_pdf_terminal_eof() {
        let cover = b"%PDF-1.4\nhello\n%%EOF\n";
        let data = b"x";
        let meta = PayloadMeta::for_text(data);
        let (stego, _) = hide(cover, "t.pdf", &meta, data, "pw").unwrap();
        assert!(stego.windows(5).rposition(|w| w == b"%%EOF").is_some());
        let out = extract(&stego, "t.pdf", "pw").unwrap();
        assert_eq!(out.data, data);
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
            lsb_depth: 1,
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

    #[test]
    fn plan_reports_capacity_risk() {
        let cover = png_cover(64, 64);
        let plan = plan_hide_with(
            &cover,
            "c.png",
            &StegoOptions::default(),
            Some(10_000),
        )
        .unwrap();
        assert!(plan.capacity_risk.is_some());
    }
}
