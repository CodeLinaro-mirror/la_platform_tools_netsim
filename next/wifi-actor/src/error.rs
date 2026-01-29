use thiserror::Error;

#[derive(Error, Debug)]
pub enum WifiError {
    #[error("Internal error: {0}")]
    Internal(String),
}
