// Copyright 2025 Google LLC
//=============================================================================
// src/types.rs - Core data types for PacketStream
//=============================================================================

use std::net::SocketAddr;
use std::path::PathBuf;

/// Stream addressing for different connection types.
///
/// This enum provides a unified way to represent addresses across different
/// transport mechanisms, allowing the same API to work with TCP sockets,
/// Unix domain sockets, gRPC endpoints, and raw file descriptors.
///
/// # Examples
///
/// ```rust
/// use packet_stream::StreamAddress;
/// use std::path::PathBuf;
///
/// // TCP address
/// let tcp_addr = StreamAddress::Tcp("127.0.0.1:8080".parse().unwrap());
///
/// // Unix domain socket
/// let uds_addr = StreamAddress::Uds(PathBuf::from("/tmp/socket.sock"));
///
/// ```
#[derive(Debug, Clone)]
pub enum StreamAddress {
    /// TCP socket address
    Tcp(SocketAddr),
    /// Unix Domain Socket path
    Uds(PathBuf),
    /// Raw file descriptors (from cuttlefish)
    Fd { in_fd: i32, out_fd: Option<i32> },
    /// gRPC endpoint address
    Grpc(SocketAddr),
}

impl std::fmt::Display for StreamAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamAddress::Uds(path) => write!(f, "UDS:{}", path.display()),
            StreamAddress::Tcp(addr) => write!(f, "TCP:{}", addr),
            StreamAddress::Fd { in_fd, out_fd } => match out_fd {
                Some(out_fd) => write!(f, "fd://{in_fd}:{out_fd}"),
                None => write!(f, "fd://{in_fd}"),
            },
            StreamAddress::Grpc(addr) => write!(f, "GRPC:{}", addr),
        }
    }
}

pub use netsim_types::{Chip, ChipInfo, ChipKind, DeviceInfo};
