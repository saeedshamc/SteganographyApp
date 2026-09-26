//! High-level hide / extract orchestration used by CLI and GUI.

use image::GenericImageView;

use crate::crypto::{decrypt_blob, encrypt_blob};
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
        CoverDecision::UseEof { .. } => Ok(HidePlan {
            method: EmbeddingMethod::Eof,
            capacity: None,
            jpeg_warning: None,
            eof_caveat: Some(EOF_CAVEAT.into()),
        }),
        CoverDecision::JpegNeedsConversion => Err(StegoError::UnsupportedCover(
            "JPEG cover detected. Lossy JPEG destroys LSB data. Convert to PNG first, or choose another cover."
                .into(),
        )),
    }
}

/// Hide a file or text payload inside a cover; returns stego file bytes and output extension hint.
pub fn hide(
    cover: &[u8],
    cover_path: &str,
    meta: &PayloadMeta,
    payload: &[u8],
    password: &str,
) -> StegoResult<(Vec<u8>, String)> {
    let plan = plan_hide(cover, cover_path)?;
    let wrapped = wrap_payload(meta, payload)?;
    let envelope = encrypt_blob(&wrapped, password)?;

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
            let png = method_a_lsb::embed(cover, &envelope, password)?;
            Ok((png, "png".into()))
        }
        EmbeddingMethod::Eof => {
            let stego = method_b_eof::embed(cover, &envelope, password)?;
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

/// Try Method B first for non-PNG, Method A for PNG/BMP; fall back if needed.
pub fn extract(stego: &[u8], stego_path: &str, password: &str) -> StegoResult<ExtractedPayload> {
    let decision = detect_from_bytes(stego, Some(stego_path))?;

    let try_lsb = || -> StegoResult<ExtractedPayload> {
        let envelope = method_a_lsb::extract(stego, password)?;
        let plain = decrypt_blob(&envelope, password)?;
        let (meta, data) = unwrap_payload(&plain)?;
        Ok(ExtractedPayload {
            meta,
            data,
            method: EmbeddingMethod::Lsb,
        })
    };

    let try_eof = || -> StegoResult<ExtractedPayload> {
        let envelope = method_b_eof::extract(stego, password)?;
        let plain = decrypt_blob(&envelope, password)?;
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
            // JPEG stego from our tool shouldn't exist; still try EOF then LSB.
            try_eof().or_else(|_| try_lsb())
        }
    }
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
}
