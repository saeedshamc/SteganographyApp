use std::fmt;

/// Result alias for stego-core operations.
pub type StegoResult<T> = Result<T, StegoError>;

/// Structured errors returned by the core (never silent failures).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StegoError {
    /// Feature not yet implemented.
    NotImplemented(&'static str),
    /// Password could not decrypt / authenticate the blob.
    WrongPassword,
    /// Ciphertext failed AES-GCM authentication (tamper or corruption).
    AuthenticationFailed,
    /// Blob too short or has an unsupported layout/version.
    InvalidFormat(String),
    /// Integrity checksum mismatch after decrypt.
    ChecksumMismatch,
    /// Payload does not fit in the cover (Method A).
    CapacityExceeded { need: usize, capacity: usize },
    /// Cover type cannot be used as requested.
    UnsupportedCover(String),
    /// Generic I/O or parsing message.
    Message(String),
}

impl fmt::Display for StegoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StegoError::NotImplemented(what) => write!(f, "not implemented: {what}"),
            StegoError::WrongPassword => write!(f, "wrong password or no hidden data found"),
            StegoError::AuthenticationFailed => {
                write!(f, "authentication failed: data corrupted or wrong password")
            }
            StegoError::InvalidFormat(msg) => write!(f, "invalid format: {msg}"),
            StegoError::ChecksumMismatch => {
                write!(f, "payload checksum mismatch: data may be corrupted")
            }
            StegoError::CapacityExceeded { need, capacity } => {
                write!(f, "payload needs {need} bytes but cover capacity is {capacity}")
            }
            StegoError::UnsupportedCover(msg) => write!(f, "unsupported cover: {msg}"),
            StegoError::Message(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for StegoError {}
