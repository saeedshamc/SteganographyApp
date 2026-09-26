use std::fmt;

/// Result alias for stego-core operations.
pub type StegoResult<T> = Result<T, StegoError>;

/// Structured errors returned by the core (never silent failures).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StegoError {
    /// Placeholder until crypto stage lands.
    NotImplemented(&'static str),
    /// Generic message for scaffolding / future mapping.
    Message(String),
}

impl fmt::Display for StegoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StegoError::NotImplemented(what) => write!(f, "not implemented: {what}"),
            StegoError::Message(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for StegoError {}
