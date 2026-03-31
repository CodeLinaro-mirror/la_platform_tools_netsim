// Copyright 2026 Google LLC

use http::header::{HeaderName, ToStrError};
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ServerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP parse error: {0}")]
    HttpParse(#[from] httparse::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] http::Error),
    #[error("Header to string error: {0}")]
    HeaderToStr(#[from] ToStrError),
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tungstenite::Error),
    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),
    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("Invalid path: {0}")]
    InvalidPath(String),
    #[error("Missing header: {0}")]
    MissingHeader(HeaderName),
    #[error("Handshake failed: {0}")]
    HandshakeFailed(String),
    #[error("Device actor error: {0}")]
    DeviceActor(#[from] netsim_model::client_error::ClientError),
}
