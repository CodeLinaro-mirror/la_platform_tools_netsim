use thiserror::Error;

#[derive(Error, Debug)]
pub enum WifiError {
    #[error("Hostapd error: {0}")]
    Hostapd(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Client error: {0}")]
    Client(String),
    #[error("Frame error: {0}")]
    Frame(String),
    #[error("Transmission error: {0}")]
    Transmission(String),
    #[error("Other error: {0}")]
    Other(String),
    #[error("Internal error: {0}")]
    Internal(String),
}
