// Copyright 2025 Google LLC
//=============================================================================
// src/error.rs - Custom error types for better error handling
//=============================================================================

use std::fmt;

/// Main error type for the PacketStream framework
#[derive(Debug)]
pub enum PacketStreamError {
    /// Socket-related errors
    Socket(SocketError),
    /// Protocol-related errors
    Protocol(ProtocolError),
    /// Configuration-related errors
    InvalidConfig(String),
    /// IO errors from underlying streams
    Io(std::io::Error),

    /// Connection was closed unexpectedly
    ConnectionClosed,
    /// Operation timed out
    Timeout,
}

#[derive(Debug)]
pub enum SocketError {
    /// Failed to bind to socket address
    BindFailed(String),
    /// Failed to connect to remote socket
    ConnectFailed(String),
    /// Socket type not supported on this platform
    UnsupportedPlatform(String),
    /// Invalid socket address format
    InvalidAddress(String),
    /// Socket accept failed
    AcceptFailed(String),
}

#[derive(Debug)]
pub enum ProtocolError {
    /// Invalid message length (too large or zero)
    InvalidLength(u32),
    /// Message size exceeds maximum allowed
    MessageTooLarge(usize),
    /// Incomplete message received
    IncompleteMessage,
    /// Invalid message format
    InvalidFormat(String),
    /// Unknown VMM identifier
    UnknownVmm(String),
}

impl fmt::Display for PacketStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PacketStreamError::Socket(e) => write!(f, "Socket error: {e}"),
            PacketStreamError::Protocol(e) => write!(f, "Protocol error: {e}"),
            PacketStreamError::InvalidConfig(e) => write!(f, "Invalid configuration: {e}"),
            PacketStreamError::Io(e) => write!(f, "IO error: {e}"),

            PacketStreamError::ConnectionClosed => write!(f, "Connection closed"),
            PacketStreamError::Timeout => write!(f, "Operation timed out"),
        }
    }
}

impl fmt::Display for SocketError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SocketError::BindFailed(addr) => write!(f, "Failed to bind to {addr}"),
            SocketError::ConnectFailed(addr) => write!(f, "Failed to connect to {addr}"),
            SocketError::UnsupportedPlatform(socket_type) => {
                write!(f, "{socket_type} sockets not supported on this platform")
            }
            SocketError::InvalidAddress(addr) => write!(f, "Invalid socket address: {addr}"),
            SocketError::AcceptFailed(reason) => write!(f, "Accept failed: {reason}"),
        }
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolError::InvalidLength(len) => write!(f, "Invalid message length: {len}"),
            ProtocolError::MessageTooLarge(size) => write!(f, "Message too large: {size} bytes"),
            ProtocolError::IncompleteMessage => write!(f, "Incomplete message received"),
            ProtocolError::InvalidFormat(msg) => write!(f, "Invalid message format: {msg}"),
            ProtocolError::UnknownVmm(vmm_id) => write!(f, "Unknown VMM identifier: {vmm_id}"),
        }
    }
}

impl std::error::Error for PacketStreamError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PacketStreamError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl std::error::Error for SocketError {}
impl std::error::Error for ProtocolError {}

// Conversions from std errors
impl From<std::io::Error> for PacketStreamError {
    fn from(err: std::io::Error) -> Self {
        PacketStreamError::Io(err)
    }
}

impl From<std::num::ParseIntError> for PacketStreamError {
    fn from(err: std::num::ParseIntError) -> Self {
        PacketStreamError::InvalidConfig(err.to_string())
    }
}

impl From<std::net::AddrParseError> for PacketStreamError {
    fn from(err: std::net::AddrParseError) -> Self {
        PacketStreamError::Socket(SocketError::InvalidAddress(err.to_string()))
    }
}

impl From<tokio::time::error::Elapsed> for PacketStreamError {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        PacketStreamError::Timeout
    }
}

impl From<tokio::task::JoinError> for PacketStreamError {
    fn from(err: tokio::task::JoinError) -> Self {
        PacketStreamError::Io(std::io::Error::new(std::io::ErrorKind::Other, err.to_string()))
    }
}

impl<T> From<tokio::sync::mpsc::error::SendError<T>> for PacketStreamError {
    fn from(err: tokio::sync::mpsc::error::SendError<T>) -> Self {
        PacketStreamError::Io(std::io::Error::new(std::io::ErrorKind::BrokenPipe, err.to_string()))
    }
}

impl From<SocketError> for PacketStreamError {
    fn from(err: SocketError) -> Self {
        PacketStreamError::Socket(err)
    }
}

impl From<ProtocolError> for PacketStreamError {
    fn from(err: ProtocolError) -> Self {
        PacketStreamError::Protocol(err)
    }
}

// Note: PacketStreamError implements std::error::Error for compatibility with
// error handling libraries

/// Result type alias for PacketStream operations
pub type Result<T> = std::result::Result<T, PacketStreamError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        assert_eq!(
            SocketError::BindFailed("localhost:8080".to_string()).to_string(),
            "Failed to bind to localhost:8080"
        );
        assert_eq!(ProtocolError::InvalidLength(10).to_string(), "Invalid message length: 10");
        assert_eq!(
            PacketStreamError::InvalidConfig("password".to_string()).to_string(),
            "Invalid configuration: password"
        );
        assert_eq!(PacketStreamError::ConnectionClosed.to_string(), "Connection closed");
    }
}
