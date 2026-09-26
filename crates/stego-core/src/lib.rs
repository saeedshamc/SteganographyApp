//! Open steganography core library.
//!
//! Module layout (filled in later stages):
//! - [`crypto`] — Argon2id key derivation and AES-256-GCM
//! - [`metadata`] — payload wrapper with integrity checksum
//! - [`detection`] — choose Method A (LSB) vs Method B (EOF)
//! - [`embedding`] — Method A and Method B implementations
//! - [`container`] — shared binary format constants

pub mod container;
pub mod crypto;
pub mod detection;
pub mod embedding;
pub mod error;
pub mod metadata;

pub use error::{StegoError, StegoResult};

/// Library version string for CLI/GUI about screens.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
