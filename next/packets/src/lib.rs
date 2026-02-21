// Copyright 2025 The Android Open Source Project

//! `packets` is a crate for zero-copy parsing and handling of network packets.
//!
//! It provides structures and utilities for working with various network
//! protocols, including Ethernet, IEEE 802.11, and Netlink messages specific to
//! `mac80211_hwsim`. The crate emphasizes performance by leveraging the
//! `zerocopy` library.
//!
//! # Testing Methodology
//!
//! The `netsim-packets` crate employs a rigorous testing strategy to ensure
//! correctness and compatibility with standard tools.
//!
//! 1. **Golden File Validation**: We use `tshark` (Wireshark) to generate
//!    "golden" JSON output for sample PCAP files. These PCAP files are
//!    typically sourced from public repositories (e.g., Wireshark Wiki) or
//!    generated via standard tools, ensuring they represent real-world traffic.
//!    Our internal tests parse the same PCAP files and verify that our JSON
//!    output matches the structure and content of the `tshark` output.
//!
//! 2. **Frame-by-Frame Tests**: Each protocol module (e.g., `ethernet`, `icmp`,
//!    `transport`) contains a `tests.rs` file that validates parsing and
//!    serialization against specific test data located in
//!    `src/<module>/test_data`. These tests use a shared utility
//!    `utils::test_utils::validate_pcap_json` to automate the comparison.
//!
//! 3. **Unit Tests**: Individual functions and structs have unit tests to
//!    verify edge cases, error handling, and specific logic (e.g., checksum
//!    calculation, header field access).
//!
//! 4. **Integration Tests**: `tests/integration_tests.rs` covers scenarios
//!    involving multiple layers or complex packet interactions.
//!
//! ## Testing with Comparison Keys
//!
//! To ensure robust validation without being overly fragile to insignificant
//! differences (like timestamps or specific capture metadata),
//! our frame-by-frame tests often use a set of **Comparison Keys**.
//!
//! Instead of comparing the entire JSON output, we validate that specific,
//! critical fields match the expected values. This allows us to verify the
//! correctness of the parsing logic for the layers we care about (e.g., 802.11,
//! LLC) while ignoring variability in the outer PCAP/PCAPNG container.
//!
//! ### Example: Layered Packet Keys
//!
//! For a packet containing **Radiotap**, **IEEE 802.11**, and **LLC/SNAP**
//! layers, we might use the following keys for validation:
//!
//! ```rust,ignore
//! let comparison_keys = &[
//!     // Radiotap (if relevant to test)
//!     "radiotap.length",
//!     "radiotap.present.tsft",
//!
//!     // IEEE 802.11
//!     "wlan.fc.type",
//!     "wlan.fc.subtype",
//!     "wlan.ra",
//!     "wlan.ta",
//!
//!     // LLC/SNAP
//!     "llc.dsap",
//!     "llc.ssap",
//!     "llc.control",
//!     "llc.oui",
//!     "llc.type",
//! ];
//! ```
//!
//! # Parsing Architecture
//!
//! This crate leverages `zerocopy` for efficient, zero-allocation parsing where
//! possible.
//!
//! * **Low-Level Parsing**: Structs derive `FromBytes` (from `zerocopy`),
//!   allowing safe casting from byte slices. Example: `let (header, payload) =
//!   IcmpHeader::parse(bytes)?;`
//! * **High-Level Parsing**:
//!     * `packet::parse`: The main entry point for **Ethernet** frames. It
//!       parses the Ethernet header and recursively parses IP and Transport
//!       layers.
//!     * `ieee80211::Ieee80211::decode`: The entry point for **IEEE 802.11**
//!       frames.
//!     * `netlink::stream::NetlinkStream::new`: The entry point for **Netlink**
//!       messages (often found in `NETLINK_ROUTE` or `NETLINK_GENERIC`
//!       sockets).
//!
//! ## Examples
//!
//! ### Parsing an Ethernet Packet
//!
//! ```rust
//! use netsim_packets::{ethernet::EthernetPacket, packet};
//!
//! let bytes = [ /* raw packet bytes */ ];
//! if let Some(packet) = packet::parse(&bytes) {
//!     let (EthernetPacket::Untagged { frame, .. } | EthernetPacket::Vlan { frame, .. }) =
//!         packet.ethernet;
//!     println!("Ethernet Destination: {}", frame.dst_addr);
//!     if let Some(ip) = &packet.ip {
//!         // Handle IP layer
//!     }
//! }
//! ```
//!
//! ### Parsing a Netlink Message
//!
//! ```rust
//! use netsim_packets::netlink::stream::NetlinkStream;
//!
//! let bytes = [ /* raw netlink bytes */ ];
//! for res in NetlinkStream::new(&bytes) {
//!     if let Ok(msg) = res {
//!         let ty = msg.0.nlmsg_type;
//!         println!("Netlink Message Type: {ty}");
//!     }
//!     // Handle attributes...
//! }
//! ```
//!
//! # JSON Serialization Architecture
//!
//! The JSON serialization in this crate is designed to produce output
//! compatible with `tshark -T json`. This facilitates direct comparison between
//! `netsim-packets` parsing and standard Wireshark parsing.
//!
//! 1. **Parsing First**: The raw bytes are first parsed into Rust structs using
//!    `zerocopy` (e.g., `Ieee80211`, `LlcSnapHeader`). This ensures that the
//!    internal representation is strongly typed and validated.
//!
//! 2. **Transformation to JSON Structs**: Each module typically has a `json.rs`
//!    submodule (e.g., `llc::json`, `netlink::nl80211_json`) that defines
//!    `serde`-compatible structs. These structs mirror the `tshark` JSON
//!    schema.
//!     * **Field Naming**: Fields are renamed using `#[serde(rename = "...")]`
//!       to match `tshark` keys (e.g., `wlan.fc.type`).
//!     * **Formatting**: Values are often formatted as strings (hex, decimal)
//!       or specific types to match `tshark`'s idiosyncrasies.
//!
//! 3. **Serialization**: The `serde_json` crate is used to serialize these
//!    intermediate structs into the final JSON string. Helper functions like
//!    `packet_to_json` or `to_json` orchestrate the assembly of multiple layers
//!    (e.g., Radiotap + 802.11 + LLC) into a single JSON object.
//!
//! ## Example: LLC/SNAP JSON
//!
//! The `llc::json` module converts an `LlcSnapHeader` into a `JsonLlc` struct,
//! which serializes to:
//!
//! ```json
//! {
//!   "llc": {
//!     "llc.dsap": 170,
//!     "llc.ssap": 170,
//!     "llc.control": 3,
//!     "llc.oui": "0x000000",
//!     "llc.type": 2048
//!   }
//! }
//! ```
#![allow(missing_docs)]
pub mod ethernet;
pub mod hci;
pub mod icmp;
pub mod ieee80211;
pub mod ip;
pub mod llc;
pub mod netlink;
pub mod packet;
pub mod pcap;
pub mod transport;
pub mod utils;
