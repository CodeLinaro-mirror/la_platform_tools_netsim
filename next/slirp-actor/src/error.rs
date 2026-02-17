use thiserror::Error;

#[derive(Error, Debug)]
pub enum SlirpError {
    #[error("LibSlirp not initialized")]
    NotInitialized,
    #[error("Internal error: {0}")]
    Internal(String),
}
