//! Cover-type detection: Method A (LSB) vs Method B (EOF).

use std::path::Path;

use crate::error::{StegoError, StegoResult};

/// Embedding strategy selected for a cover file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingMethod {
    /// Keyed random LSB for PNG/BMP covers.
    Lsb,
    /// Keyed EOF append for generic covers.
    Eof,
}

impl EmbeddingMethod {
    pub fn as_str(self) -> &'static str {
        match self {
            EmbeddingMethod::Lsb => "Method A (LSB)",
            EmbeddingMethod::Eof => "Method B (EOF append)",
        }
    }
}

/// How JPEG covers should be handled when asked to hide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JpegPolicy {
    /// Reject JPEG covers with a clear error.
    Reject,
    /// Allow hide only after the caller converts to PNG (detection still reports ConvertJpeg).
    ConvertToPng,
}

/// Result of inspecting a cover path / bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoverDecision {
    /// Use Method A; capacity can be computed from decoded dimensions.
    UseLsb { extension: String },
    /// Use Method B.
    UseEof { extension: String },
    /// JPEG is lossy — cannot LSB safely without conversion.
    JpegNeedsConversion,
}

fn extension_of(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_default()
}

/// Detect method from a file path (extension-based).
pub fn detect_method(path: &str) -> StegoResult<EmbeddingMethod> {
    match detect_cover(path, JpegPolicy::Reject)? {
        CoverDecision::UseLsb { .. } => Ok(EmbeddingMethod::Lsb),
        CoverDecision::UseEof { .. } => Ok(EmbeddingMethod::Eof),
        CoverDecision::JpegNeedsConversion => Err(StegoError::UnsupportedCover(
            "JPEG covers destroy LSB data under lossy compression; use PNG/BMP, or convert to PNG"
                .into(),
        )),
    }
}

/// Full cover decision including JPEG caveat.
pub fn detect_cover(path: &str, jpeg_policy: JpegPolicy) -> StegoResult<CoverDecision> {
    let ext = extension_of(path);
    match ext.as_str() {
        "png" | "bmp" => Ok(CoverDecision::UseLsb { extension: ext }),
        "jpg" | "jpeg" | "jpe" | "jfif" => match jpeg_policy {
            JpegPolicy::Reject => Err(StegoError::UnsupportedCover(
                "JPEG is lossy and unsuitable for LSB steganography; convert to PNG or pick another cover"
                    .into(),
            )),
            JpegPolicy::ConvertToPng => Ok(CoverDecision::JpegNeedsConversion),
        },
        _ => Ok(CoverDecision::UseEof { extension: ext }),
    }
}

/// Sniff magic bytes when the extension is missing or ambiguous.
pub fn detect_from_bytes(data: &[u8], hinted_path: Option<&str>) -> StegoResult<CoverDecision> {
    if data.len() >= 8 && &data[..8] == b"\x89PNG\r\n\x1a\n" {
        return Ok(CoverDecision::UseLsb {
            extension: "png".into(),
        });
    }
    if data.len() >= 2 && data[0] == b'B' && data[1] == b'M' {
        return Ok(CoverDecision::UseLsb {
            extension: "bmp".into(),
        });
    }
    if data.len() >= 3 && data[0] == 0xff && data[1] == 0xd8 && data[2] == 0xff {
        return Ok(CoverDecision::JpegNeedsConversion);
    }
    if let Some(path) = hinted_path {
        return detect_cover(path, JpegPolicy::ConvertToPng);
    }
    Ok(CoverDecision::UseEof {
        extension: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_and_bmp_use_lsb() {
        assert_eq!(
            detect_method("photo.PNG").unwrap(),
            EmbeddingMethod::Lsb
        );
        assert_eq!(
            detect_method(r"C:\x\cover.bmp").unwrap(),
            EmbeddingMethod::Lsb
        );
    }

    #[test]
    fn jpeg_rejected_by_default() {
        let err = detect_method("x.jpg").unwrap_err();
        match err {
            StegoError::UnsupportedCover(msg) => assert!(msg.to_lowercase().contains("jpeg")),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn jpeg_convert_policy() {
        assert_eq!(
            detect_cover("x.jpeg", JpegPolicy::ConvertToPng).unwrap(),
            CoverDecision::JpegNeedsConversion
        );
    }

    #[test]
    fn generic_files_use_eof() {
        assert_eq!(detect_method("doc.pdf").unwrap(), EmbeddingMethod::Eof);
        assert_eq!(detect_method("track.mp3").unwrap(), EmbeddingMethod::Eof);
        assert_eq!(detect_method("a.zip").unwrap(), EmbeddingMethod::Eof);
        assert_eq!(detect_method("notes.docx").unwrap(), EmbeddingMethod::Eof);
    }

    #[test]
    fn magic_sniff_png() {
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend_from_slice(&[0u8; 16]);
        match detect_from_bytes(&png, None).unwrap() {
            CoverDecision::UseLsb { extension } => assert_eq!(extension, "png"),
            other => panic!("{other:?}"),
        }
    }
}
