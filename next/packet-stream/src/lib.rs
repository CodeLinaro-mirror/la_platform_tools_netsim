// Copyright 2025 Google LLC
//=============================================================================
// src/lib.rs - Public API exports
//=============================================================================

//! # PacketStream
//!
//! High-performance, cross-platform packet streaming framework for virtualized
//! environments. Transports network packets (802.11, 802.3, UWB UCI, Bluetooth
//! HCI) between guest and host systems with automatic device information
//! exchange.
//!
//! ## Features
//!
//! - **Zero-copy operations** - `Bytes`-based reference counting with
//!   allocation tracking
//! - **Cross-platform** - Unix Domain Sockets, Named Pipes, VSOCK, TCP with
//!   auto-detection
//! - **Transport-agnostic** - Unified `Streams` API across all transport types
//! - **Performance** - >1.3 Gbps throughput, <12μs latency on modern hardware
//!
//! ## Quick Start
//!
//!
//! ## Wire Format
//!
//! Simple length-prefixed binary protocol: `[4-byte length][payload]`.
//! Uses native byte order for local IPC efficiency.
//!
//! ## Performance
//!
//! Benchmarked on Apple M2 Mac mini:
//!
//! | Transport | Bandwidth | Latency | Use Case |
//! |-----------|-----------|---------|----------|
//! | **UDS** | 1.3+ Gbps | 11.7μs | Local IPC |
//! | **TCP** | 800+ Mbps | 15-20μs | Network fallback |
//!
//! Supports 802.11, 802.3, Bluetooth HCI, and UWB UCI packet sizes efficiently.
//!
//! ## Platform Support
//!
//! - **Unix/Linux/macOS**: Unix Domain Sockets (best performance)
//! - **Windows**: Named Pipes with TCP fallback
//! - **VMs**: VSOCK for guest-host communication
//! - **Containers**: Volume mounts or port forwarding

// Benchmark module moved to packetstream-bins crate
//! Netsim Packet Stream
//!
//! This crate provides the core transport and streaming abstractions for
//! Netsim. It handles the connection handshake, packet framing, and
//! multiplexing of packet streams over various transports (TCP, UDS, gRPC,
//! etc.).
//!
//! The main entry points are:
//! - `CrossPlatformListener`: For listening for incoming connections.
//! - `CrossPlatformStream`: For connecting to a remote endpoint.
//! - `PacketStream` / `PacketSink`: Traits for reading and writing packets.

pub mod error; // Error types
pub mod streams; // High-level connection management
pub mod transport; // Transport implementations (includes socket)
pub mod types; // Core data types

// Core public API - only expose what users actually need
pub use error::{PacketStreamError, ProtocolError, SocketError};
// Streams architecture re-exports - Streams is the only public entry point
pub use streams::{InitInfo, Streams};
#[cfg(all(unix, feature = "dual_fd"))]
pub use transport::dual_fd::{ChipConfig, DeviceConfig, DualFdConfig};
// Core streaming types - public API only (transport internals hidden)
pub use transport::TransportType;
pub use transport::{
    CrossPlatformListener, CrossPlatformStream, ListenerConfig, SocketConfig, SocketType,
};
pub use types::StreamAddress;

pub use crate::types::{Chip, ChipInfo, ChipKind, DeviceInfo};
