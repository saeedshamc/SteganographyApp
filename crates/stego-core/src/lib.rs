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
pub mod ops;
pub mod paths;

pub use crypto::{derive_keys_with, CryptoOptions, KdfProfile, SALT_LEN};
pub use detection::EmbeddingMethod;
pub use error::{StegoError, StegoResult};
pub use metadata::{is_executable_extension, PayloadKind, PayloadMeta};
pub use ops::{extract, extract_with, hide, hide_with, plan_hide, parse_kdf_profile, ExtractedPayload, HidePlan, StegoOptions};

/// Library version string for CLI/GUI about screens.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
