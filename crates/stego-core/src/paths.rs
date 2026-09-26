//! Shared notes for desktop / CLI path handling (Windows + Linux).

/// Documented behavior (no runtime deps):
/// - Paths are accepted as provided by the OS / file dialogs (`/` or `\`).
/// - Parent directories for CLI `--output` are created when missing.
/// - GUI save/open uses native dialogs via `rfd` (works on Windows and Linux
///   with a desktop portal / Zenity / Windows common dialogs).
/// - Never rewrite user paths to a different drive/root without asking.

#[cfg(test)]
mod tests {
    use std::path::Path;

    #[test]
    fn path_display_roundtrip() {
        let p = Path::new("subdir").join("out.png");
        assert!(p.to_string_lossy().contains("out.png"));
    }
}
