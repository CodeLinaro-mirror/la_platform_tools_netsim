use netsim_model::ChipId;
use thiserror::Error;

/// Error type for CaptureActor operations.
#[derive(Error, Debug)]
pub enum CaptureError {
    /// IO error during file operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Anyhow error for generic errors.
    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
    /// Chip not found.
    #[error("Chip not found: {0:?}")]
    ChipNotFound(ChipId),
    /// Chip Kind Mismatch
    #[error("Chip Kind Mismatch: expected {0}, found {1}")]
    ChipKindMismatch(String, String),
}

impl From<String> for CaptureError {
    fn from(s: String) -> Self {
        CaptureError::Anyhow(anyhow::anyhow!(s))
    }
}
