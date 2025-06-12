//! Defines structures for representing IEEE 802.2 Logical Link Control (LLC)
//! and Subnetwork Access Protocol (SNAP) headers using `zerocopy`.
//!
//! This module provides `zerocopy`-based structures for parsing LLC and SNAP
//! headers commonly found in Ethernet frames, especially when non-IP protocols
//! are encapsulated.
//!
//! - [`LlcHeader`]: Represents the basic 3-byte LLC header.
//! - [`SnapHeader`]: Represents the 5-byte SNAP header that often follows an LLC header.
//! - [`LlcSnapHeader`]: A combined structure for parsing both LLC and SNAP headers together.

use zerocopy::byteorder::NetworkEndian;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned, U16};

/// LLC Service Access Point (SAP) address constants.
pub mod sap {
    /// Null LSAP Address.
    pub const NULL: u8 = 0x00;
    /// Individual LLC Sublayer Management function.
    pub const ILMI: u8 = 0x04;
    /// Global DSAP (Group Address).
    pub const GLOBAL: u8 = 0xFF;
    /// Subnetwork Access Protocol (SNAP).
    pub const SNAP: u8 = 0xAA;
    /// Spanning Tree Protocol (BPDU).
    pub const STP: u8 = 0x42;
    // Other common SAPs can be added here.
}

/// LLC Control Field constants.
/// This focuses on Unnumbered Information (UI) frames, common with SNAP.
pub mod control_field {
    /// Unnumbered Information (UI) frame.
    pub const UI: u8 = 0x03;
    /// Exchange Identification (XID) frame.
    pub const XID: u8 = 0xAF; // Or 0xBF for response
    /// Test frame.
    pub const TEST: u8 = 0xE3; // Or 0xF3 for response
}

/// Represents an IEEE 802.2 LLC (Logical Link Control) header.
/// This basic version assumes a 1-byte control field, common for UI frames.
///
/// LLC is a sublayer of the Data Link Layer in the OSI model and is specified
/// by IEEE 802.2. It provides a way to multiplex different network layer protocols
/// over the same physical medium.
///
/// # Examples
///
/// ```
/// use packets_zc::llc::{LlcHeader, sap, control_field};
///
/// // Create an LLC header for SNAP
/// let llc_hdr = LlcHeader::new(sap::SNAP, sap::SNAP, control_field::UI);
/// assert_eq!(llc_hdr.dsap, 0xAA);
/// ```
#[repr(C)]
#[derive(
    FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout, Debug, Copy, Clone, PartialEq, Eq,
)]
pub struct LlcHeader {
    /// Destination Service Access Point (DSAP).
    pub dsap: u8,
    /// Source Service Access Point (SSAP).
    pub ssap: u8,
    /// Control field (1 byte for UI, XID, TEST frames).
    pub control: u8,
}

impl LlcHeader {
    /// Creates a new `LlcHeader`.
    pub const fn new(dsap: u8, ssap: u8, control: u8) -> Self {
        Self { dsap, ssap, control }
    }
}

/// Represents an IEEE 802.2 SNAP (Subnetwork Access Protocol) header.
/// This header follows the LLC header when DSAP and SSAP are `0xAA`.
///
/// SNAP extends the LLC protocol by providing a way to encapsulate protocols
/// that do not have their own SAP values assigned, often using EtherType-like
/// Protocol Identifiers (PIDs).
///
/// # Examples
///
/// ```
/// use packets_zc::llc::SnapHeader;
/// use packets_zc::ethernet::ether_type; // For common PIDs
///
/// let snap_hdr = SnapHeader::new([0x00, 0x00, 0x0C], ether_type::IPV4); // Cisco OUI, IPv4 PID
/// assert_eq!(snap_hdr.oui, [0x00, 0x00, 0x0C]);
/// assert_eq!(snap_hdr.protocol_id.get(), 0x0800);
/// ```
#[repr(C)]
#[derive(
    FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout, Debug, Copy, Clone, PartialEq, Eq,
)]
pub struct SnapHeader {
    /// Organizationally Unique Identifier (OUI) (3 bytes).
    pub oui: [u8; 3],
    /// Protocol Identifier (PID) (2 bytes), similar to EtherType.
    /// Stored in network byte order (big-endian). Use `.get()` to retrieve
    /// the value in host byte order.
    pub protocol_id: U16<NetworkEndian>,
}

impl SnapHeader {
    /// Creates a new `SnapHeader`.
    pub const fn new(oui: [u8; 3], protocol_id_val: u16) -> Self {
        Self { oui, protocol_id: U16::new(protocol_id_val) }
    }
}

/// Represents a combined LLC and SNAP header.
/// This is common when Ethernet frames carry non-IP protocols over LLC.
///
/// When an Ethernet frame's EtherType field indicates a length (rather than a
/// protocol type), it often implies that an LLC header follows. If that LLC
/// header has DSAP and SSAP set to `0xAA` (SNAP SAP), then a SNAP header
/// will follow the LLC header. This combined structure facilitates parsing
/// such common encapsulations.
///
/// # Examples
///
/// Parsing an LLC+SNAP header from a byte slice:
/// ```
/// use packets_zc::llc::{LlcSnapHeader, sap, control_field};
/// use packets_zc::ethernet::ether_type;
/// use zerocopy::Ref;
///
/// let bytes: [u8; 8] = [0xAA, 0xAA, 0x03, 0x00, 0x00, 0x0C, 0x08, 0x00]; // LLC+SNAP for IPv4
/// let (header_ref, _rest) = Ref::<&[u8], LlcSnapHeader>::from_prefix(&bytes[..]).unwrap();
///
/// assert_eq!(header_ref.llc.dsap, sap::SNAP);
/// assert_eq!(header_ref.snap.protocol_id.get(), ether_type::IPV4);
/// ```
#[repr(C)]
#[derive(
    FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout, Debug, Copy, Clone, PartialEq, Eq,
)]
pub struct LlcSnapHeader {
    /// The LLC header part.
    pub llc: LlcHeader,
    /// The SNAP header part.
    pub snap: SnapHeader,
}

impl LlcSnapHeader {
    /// Creates a new `LlcSnapHeader`.
    ///
    /// Typically, for SNAP, `dsap` and `ssap` are `sap::SNAP` (0xAA),
    /// and `control` is `control_field::UI` (0x03).
    pub const fn new(dsap: u8, ssap: u8, control: u8, oui: [u8; 3], protocol_id_val: u16) -> Self {
        Self {
            llc: LlcHeader::new(dsap, ssap, control),
            snap: SnapHeader::new(oui, protocol_id_val),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ethernet::ether_type; // For example PID values
    use core::mem::size_of;
    use zerocopy::Ref;

    #[test]
    fn test_llc_header_size() {
        assert_eq!(size_of::<LlcHeader>(), 3, "LlcHeader size should be 3 bytes");
    }

    #[test]
    fn test_snap_header_size() {
        assert_eq!(size_of::<SnapHeader>(), 5, "SnapHeader size should be 5 bytes");
    }

    #[test]
    fn test_llc_snap_header_size() {
        assert_eq!(size_of::<LlcSnapHeader>(), 8, "LlcSnapHeader size should be 8 bytes");
    }

    #[test]
    fn test_llc_snap_header_new() {
        let oui = [0x00, 0x00, 0x0C]; // Cisco OUI example
        let pid = ether_type::IPV4;
        let header = LlcSnapHeader::new(sap::SNAP, sap::SNAP, control_field::UI, oui, pid);

        assert_eq!(header.llc.dsap, sap::SNAP);
        assert_eq!(header.llc.ssap, sap::SNAP);
        assert_eq!(header.llc.control, control_field::UI);
        assert_eq!(header.snap.oui, oui);
        assert_eq!(header.snap.protocol_id.get(), pid);
    }

    #[test]
    fn test_llc_snap_header_parsing() {
        // LLC (AA:AA:03) + SNAP (OUI: 00-80-C2, PID: 0x000E - STP variant)
        let bytes: [u8; 8] = [
            0xAA, // DSAP (SNAP)
            0xAA, // SSAP (SNAP)
            0x03, // Control (UI)
            0x00, 0x80, 0xC2, // OUI
            0x00, 0x0E, // PID
        ];

        let (header, rest) = Ref::<&[u8], LlcSnapHeader>::from_prefix(&bytes[..])
            .expect("Should successfully parse LlcSnapHeader");

        assert!(rest.is_empty(), "Expected no remaining bytes");
        assert_eq!(header.llc.dsap, sap::SNAP);
        assert_eq!(header.llc.ssap, sap::SNAP);
        assert_eq!(header.llc.control, control_field::UI);
        assert_eq!(header.snap.oui, [0x00, 0x80, 0xC2]);
        assert_eq!(header.snap.protocol_id.get(), 0x000E);

        // Test with insufficient slice
        let short_bytes: &[u8] = &[0xAA, 0xAA, 0x03];
        assert!(
            Ref::<&[u8], LlcSnapHeader>::from_prefix(short_bytes).is_err(),
            "Should fail to parse too short slice"
        );
    }
}
