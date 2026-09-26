//! Shared container / format constants.
//!
//! Byte-level layout is documented in `docs/FORMAT.md` and implemented in later stages.

/// Human-readable format identifier used in docs and diagnostics.
pub const FORMAT_NAME: &str = "OpenStego";

/// Format version byte written into encrypted envelopes (stage 2+).
pub const FORMAT_VERSION: u8 = 1;
