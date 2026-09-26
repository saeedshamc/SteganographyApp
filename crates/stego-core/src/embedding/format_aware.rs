//! Format-aware cover preparation for Method B (EOF), with safe fallbacks.
//!
//! Strategy:
//! - **PDF**: if a `%%EOF` marker sits in the last 16 bytes, strip it before append and
//!   re-attach after the Method B footer so many viewers still see a terminal marker.
//! - **ZIP / MP3 / other**: leave bytes unchanged; Method B plain append (EOF fallback).
//!   Callers surface format-specific caveats via [`format_caveat`].

use crate::error::StegoResult;

const EOF_MARK: &[u8] = b"%%EOF";

/// Human-readable caveat for Method B covers.
pub fn format_caveat(cover: &[u8], cover_path: &str) -> String {
    let ext = extension(cover_path);
    if ext == "pdf" || cover.starts_with(b"%PDF") {
        return "PDF: Method B appends after the file. If %%EOF was at the very end, it is moved after the stego footer so many viewers still open the document.".into();
    }
    if ext == "zip" || ext == "jar" || looks_like_zip(cover) {
        return "ZIP/JAR: trailing Method B bytes are ignored by most unzip tools, but strict validators may complain.".into();
    }
    if ext == "mp3" || looks_like_mp3(cover) {
        return "MP3: most players ignore trailing bytes after the last frame; capacity is effectively unbounded.".into();
    }
    "Method B appends data after the file end. Most formats ignore trailing bytes, but a few with strict validation may reject the output.".into()
}

/// Prepare cover bytes before Method B embed. Always succeeds (falls back to original).
pub fn prepare_cover(cover: &[u8], cover_path: &str) -> Vec<u8> {
    if is_pdf(cover, cover_path) {
        if let Some(pos) = terminal_eof_pos(cover) {
            return cover[..pos].to_vec();
        }
    }
    cover.to_vec()
}

/// Finalize stego bytes after Method B embed (e.g. restore PDF %%EOF).
pub fn finalize_stego(stego: Vec<u8>, original_cover: &[u8], cover_path: &str) -> Vec<u8> {
    if !is_pdf(original_cover, cover_path) {
        return stego;
    }
    // Only restore if we stripped a terminal marker (original had %%EOF near end).
    if terminal_eof_pos(original_cover).is_none() {
        return stego;
    }
    if terminal_eof_pos(&stego).is_some() {
        return stego;
    }
    let mut out = stego;
    out.extend_from_slice(b"\n%%EOF\n");
    out
}

/// Strip a terminal PDF %%EOF so Method B footer parsing sees the real length field.
pub fn strip_for_extract(stego: &[u8], stego_path: &str) -> Vec<u8> {
    if !is_pdf(stego, stego_path) && !stego.starts_with(b"%PDF") {
        return stego.to_vec();
    }
    let mut out = if let Some(pos) = terminal_eof_pos(stego) {
        stego[..pos].to_vec()
    } else {
        stego.to_vec()
    };
    // Drop whitespace inserted before a restored %%EOF marker.
    while out.last().is_some_and(|b| matches!(b, b'\n' | b'\r' | b' ')) {
        out.pop();
    }
    out
}

fn is_pdf(cover: &[u8], path: &str) -> bool {
    extension(path) == "pdf" || cover.starts_with(b"%PDF")
}

fn extension(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn terminal_eof_pos(data: &[u8]) -> Option<usize> {
    data.windows(EOF_MARK.len())
        .rposition(|w| w == EOF_MARK)
        .filter(|pos| data.len() - pos <= 16)
}

fn looks_like_zip(data: &[u8]) -> bool {
    data.len() >= 4 && &data[..2] == b"PK"
}

fn looks_like_mp3(data: &[u8]) -> bool {
    (data.len() >= 3 && &data[..3] == b"ID3")
        || (data.len() >= 2 && data[0] == 0xff && (data[1] & 0xe0) == 0xe0)
}

/// Capacity risk note when payload approaches image capacity (educational).
pub fn capacity_risk_note(need: usize, capacity: usize) -> Option<String> {
    if capacity == 0 {
        return Some("Image capacity is zero for this size.".into());
    }
    let pct = (need * 100) / capacity;
    if pct >= 85 {
        Some(format!(
            "High capacity use (~{pct}% of {capacity} bytes). Larger images or Method B covers are safer."
        ))
    } else if pct >= 60 {
        Some(format!(
            "Moderate capacity use (~{pct}% of {capacity} bytes)."
        ))
    } else {
        None
    }
}

/// Validate requested LSB depth (1 = classic, 2 = denser / noisier educational mode).
pub fn normalize_lsb_depth(depth: u8) -> StegoResult<u8> {
    match depth {
        0 | 1 => Ok(1),
        2 => Ok(2),
        d => Err(crate::error::StegoError::Message(format!(
            "lsb depth must be 1 or 2 (got {d})"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pdf_terminal_eof_stripped_and_restored() {
        let cover = b"%PDF-1.4\ncontent\n%%EOF\n";
        assert!(terminal_eof_pos(cover).is_some());
        let prep = prepare_cover(cover, "a.pdf");
        assert!(!prep.windows(5).any(|w| w == b"%%EOF"));
        let fake_stego = {
            let mut v = prep.clone();
            v.extend_from_slice(b"ENVELOPE");
            v
        };
        let out = finalize_stego(fake_stego, cover, "a.pdf");
        assert!(out.ends_with(b"%%EOF\n"));
    }

    #[test]
    fn pdf_eof_not_at_end_left_alone() {
        let mut cover = b"%PDF-1.4\n%%EOF\n".to_vec();
        cover.extend_from_slice(&[1u8; 64]);
        let prep = prepare_cover(&cover, "a.pdf");
        assert_eq!(prep, cover);
    }

    #[test]
    fn capacity_risk_thresholds() {
        assert!(capacity_risk_note(90, 100).unwrap().contains("High"));
        assert!(capacity_risk_note(65, 100).unwrap().contains("Moderate"));
        assert!(capacity_risk_note(10, 100).is_none());
    }
}
