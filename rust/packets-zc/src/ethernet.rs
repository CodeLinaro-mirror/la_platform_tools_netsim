//! Defines structures for representing Ethernet II frames using `zerocopy` for zero-copy parsing.
//!
//! This module provides definitions for MAC addresses, EtherType constants, and the
//! Ethernet II frame header, suitable for high-performance network packet analysis.

use core::fmt;
use zerocopy::byteorder::NetworkEndian;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned, U16};

/// Represents a 6-byte MAC address.
#[repr(C)]
#[derive(
    Debug, Copy, Clone, PartialEq, Eq, FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout,
)]
pub struct MacAddr {
    /// The 6 bytes of the MAC address.
    pub bytes: [u8; 6],
}

impl MacAddr {
    /// Creates a new `MacAddr` from a 6-byte array.
    pub const fn new(bytes: [u8; 6]) -> Self {
        Self { bytes }
    }
}

impl fmt::Display for MacAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.bytes[0],
            self.bytes[1],
            self.bytes[2],
            self.bytes[3],
            self.bytes[4],
            self.bytes[5]
        )
    }
}

/// EtherType constants representing common L3 protocols.
///
/// These values are used in the EtherType field of an Ethernet frame
/// to indicate the protocol of the encapsulated payload.
/// Values are standard `u16` and should be compared against the
/// host-endian value retrieved from `EthernetFrame.ethertype.get()`.
pub mod ether_type {
    /// EtherType for IPv4 (Internet Protocol version 4).
    pub const IPV4: u16 = 0x0800;
    /// EtherType for ARP (Address Resolution Protocol).
    pub const ARP: u16 = 0x0806;
    /// EtherType for IPv6 (Internet Protocol version 6).
    pub const IPV6: u16 = 0x86DD;
    /// EtherType for VLAN-tagged frames (IEEE 802.1Q).
    pub const VLAN: u16 = 0x8100;
}

/// Represents an Ethernet II frame header.
///
/// This struct is designed for zero-copy parsing of raw packet data.
/// It uses `#[repr(C)]` to ensure a C-compatible memory layout,
/// and derives `FromBytes`, `IntoBytes`, `Unaligned`, `Immutable`,
/// and `KnownLayout` from `zerocopy`
/// to allow safe casting from a byte slice.
#[repr(C)]
#[derive(FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout)]
pub struct EthernetFrame {
    /// Destination MAC address (6 bytes).
    pub dst_addr: MacAddr,
    /// Source MAC address (6 bytes).
    pub src_addr: MacAddr,
    /// EtherType field (2 bytes), indicating the protocol of the payload.
    /// Stored in network byte order (big-endian). Use `.get()` to retrieve
    /// the value in host byte order.
    pub ethertype: U16<NetworkEndian>,
}

impl EthernetFrame {
    /// Creates a new `EthernetFrame`.
    ///
    /// # Arguments
    /// * `dst_addr` - The destination MAC address.
    /// * `src_addr` - The source MAC address.
    /// * `ethertype_val` - The EtherType value (e.g., `ether_type::IPV4`).
    pub fn new(dst_addr: MacAddr, src_addr: MacAddr, ethertype_val: u16) -> Self {
        Self { dst_addr, src_addr, ethertype: U16::new(ethertype_val) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem::size_of;
    use zerocopy::Ref; // Updated to use Ref as per zerocopy 0.8.x

    /// Tests that the size of the EthernetFrame struct is correct.
    #[test]
    fn test_ethernet_frame_size() {
        assert_eq!(size_of::<EthernetFrame>(), 14, "EthernetFrame size should be 14 bytes");
    }

    /// Tests that a MacAddr can be displayed correctly.
    #[test]
    fn test_mac_addr_new() {
        let bytes = [0x11, 0x22, 0x33, 0xAA, 0xBB, 0xCC];
        let mac = MacAddr::new(bytes);
        assert_eq!(mac.bytes, bytes);
    }

    /// Tests the EthernetFrame::new constructor.
    #[test]
    fn test_ethernet_frame_new() {
        let dst_mac = MacAddr::new([0x01, 0x02, 0x03, 0x04, 0x05, 0x06]);
        let src_mac = MacAddr::new([0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F]);
        let eth_type = ether_type::IPV6;
        let frame = EthernetFrame::new(dst_mac, src_mac, eth_type);
        assert_eq!(frame.dst_addr, dst_mac);
        assert_eq!(frame.src_addr, src_mac);
        assert_eq!(frame.ethertype.get(), eth_type);
    }

    #[test]
    fn test_mac_addr_display() {
        let mac = MacAddr { bytes: [0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02] };
        assert_eq!(format!("{}", mac), "DE:AD:BE:EF:01:02");
    }

    /// Tests parsing a valid Ethernet frame byte slice using LayoutVerified.
    #[test]
    fn test_ethernet_frame_parsing() {
        // Sample Ethernet frame:
        // Dst MAC: DE:AD:BE:EF:00:01
        // Src MAC: C0:DE:FE:ED:00:02
        // EtherType: IPv4 (0x0800)
        let frame_bytes: [u8; 14] = [
            0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01, // Dst MAC
            0xC0, 0xDE, 0xFE, 0xED, 0x00, 0x02, // Src MAC
            0x08, 0x00, // EtherType (IPv4)
        ];

        // Attempt to view the byte slice as an EthernetFrame
        let (frame, rest) = Ref::<&[u8], EthernetFrame>::from_prefix(&frame_bytes[..])
            .expect("Should successfully parse the frame prefix");

        assert!(
            rest.is_empty(),
            "Expected no remaining bytes after parsing prefix from exact-size input"
        );

        // Verify destination MAC address
        assert_eq!(
            frame.dst_addr.bytes,
            [0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01],
            "Destination MAC address mismatch"
        );

        // Verify source MAC address
        assert_eq!(
            frame.src_addr.bytes,
            [0xC0, 0xDE, 0xFE, 0xED, 0x00, 0x02],
            "Source MAC address mismatch"
        );

        // Verify EtherType (converting from network to host byte order)
        assert_eq!(frame.ethertype.get(), ether_type::IPV4, "EtherType mismatch");

        // Test with a slice that is exactly the size of the frame
        let exact_frame_bytes: &[u8] = &[
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x08,
            0x06, // ARP
        ];
        let frame = Ref::<_, EthernetFrame>::from_bytes(exact_frame_bytes)
            .expect("Should successfully parse exact size frame");
        assert_eq!(frame.dst_addr.bytes, [0x01, 0x02, 0x03, 0x04, 0x05, 0x06],);
        assert_eq!(frame.src_addr.bytes, [0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C],);
        assert_eq!(frame.ethertype.get(), ether_type::ARP);

        // Test with an insufficient slice
        let short_bytes: &[u8] = &[0xDE, 0xAD, 0xBE, 0xEF, 0x00]; // Too short
        let short_result = Ref::<&[u8], EthernetFrame>::from_prefix(short_bytes);
        assert!(short_result.is_err(), "Should fail to parse too short slice");

        let short_result_exact = Ref::<&[u8], EthernetFrame>::from_bytes(short_bytes);
        assert!(
            short_result_exact.is_err(),
            "Should fail to parse too short slice with from_bytes"
        );
    }

    #[test]
    fn test_ethernet_frame_parsing_with_remainder() {
        // Frame + extra data
        let frame_bytes_extra: [u8; 16] = [
            0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x01, // Dst MAC
            0xC0, 0xDE, 0xFE, 0xED, 0x00, 0x02, // Src MAC
            0x08, 0x00, // EtherType (IPv4)
            0xFF, 0xFF, // Extra data
        ];

        let (frame, rest) = Ref::<&[u8], EthernetFrame>::from_prefix(&frame_bytes_extra[..])
            .expect("Should successfully parse the frame prefix with remainder");

        assert_eq!(frame.ethertype.get(), ether_type::IPV4);
        assert_eq!(rest.len(), 2, "Expected 2 remaining bytes");
        assert_eq!(rest, &[0xFF, 0xFF]);
    }
}
