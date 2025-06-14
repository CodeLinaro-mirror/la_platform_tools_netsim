# `packets_zc`

`packets_zc` is a crate for zero-copy parsing and handling of network packets.
It provides structures and utilities for working with various network protocols,
leveraging the `zerocopy` library for high-performance, allocation-free parsing
where possible.

## Features

- Zero-copy parsing of packet headers.
- Structures for common network protocols.
- Utility functions for interpreting header fields.
- Optional JSON serialization/deserialization for some protocol headers, often
  mimicking `tshark` field names.

## Supported Protocols and Structures

### Ethernet II Frames

The `ethernet` module defines structures for Ethernet II frames:
- `MacAddr`: Represents a 6-byte MAC address.
- `EthernetFrame`: Represents the 14-byte Ethernet II header (Destination MAC, Source MAC, EtherType).
- EtherType constants are provided in `ethernet::ether_type`.

The `ethernet_util` module provides utility functions like `ethertype_to_string` and MAC address type checks (broadcast, multicast, unicast).
The `ethernet_json` module supports JSON serialization/deserialization of Ethernet headers.

### IEEE 802.11 (Wi-Fi) MAC Headers

The `ieee80211` module provides structures for common IEEE 802.11 MAC headers:
- `FrameControl`: Represents the 2-byte Frame Control field.
- `SequenceControl`: Represents the 2-byte Sequence Control field.
- `MacHeader3Addr`: A generic 24-byte MAC header with 3 address fields, common for many frame types.
- Specific header structures like `DataFrameHeader` and `BeaconFrameHeader`.
- Constants for frame types and subtypes (e.g., `ieee80211::frame_type::DATA`, `ieee80211::management_subtype::BEACON`).

The `ieee80211_util` module offers utilities for interpreting Frame Control fields and determining address roles (RA, TA, DA, SA, BSSID) based on ToDS/FromDS flags.
The `ieee80211_json` module provides `serde`-compatible structures for JSON representation of 802.11 headers.

### IEEE 802.2 LLC and SNAP Headers

The `llc` module provides support for parsing IEEE 802.2 Logical Link Control (LLC) and Subnetwork Access Protocol (SNAP) headers. These headers are often found in Ethernet frames when encapsulating non-IP protocols or when the EtherType field is used as a length field.

-   **`LlcHeader`**: Represents the 3-byte LLC header (DSAP, SSAP, Control).
-   **`SnapHeader`**: Represents the 5-byte SNAP header (OUI, Protocol ID), which typically follows an LLC header when DSAP and SSAP are `0xAA`.
-   **`LlcSnapHeader`**: A combined structure for parsing both LLC and SNAP headers together (8 bytes).
-   Constants for common SAP values (e.g., `llc::sap::SNAP`) and control field values (e.g., `llc::control_field::UI`) are provided.

**Example: Parsing an LLC+SNAP Header**
```rust
use packets_zc::llc::{LlcSnapHeader, sap, control_field};
use packets_zc::ethernet::ether_type; // For example PID
use zerocopy::Ref;

// LLC (SNAP SAPs, UI control) + SNAP (Example OUI, IPv4 PID)
let bytes: [u8; 8] = [
    0xAA,       // DSAP (SNAP)
    0xAA,       // SSAP (SNAP)
    0x03,       // Control (UI)
    0x00, 0x00, 0x0C, // OUI (e.g., Cisco)
    0x08, 0x00, // Protocol ID (IPv4)
];

if let Some((header_ref, _rest)) = Ref::<&[u8], LlcSnapHeader>::from_prefix(&bytes[..]) {
    assert_eq!(header_ref.llc.dsap, sap::SNAP);
    assert_eq!(header_ref.llc.ssap, sap::SNAP);
    assert_eq!(header_ref.llc.control, control_field::UI);
    assert_eq!(header_ref.snap.oui, [0x00, 0x00, 0x0C]);
    assert_eq!(header_ref.snap.protocol_id.get(), ether_type::IPV4);
}
```

The `llc_util` module offers helper functions, such as converting SAP and control field values to strings.
The `llc_json` module provides `serde`-compatible structures and functions for JSON serialization and deserialization of LLC/SNAP headers, mirroring `tshark` field names.

### `mac80211_hwsim` Netlink Messages

The `mac80211_hwsim_netlink` module provides structures for parsing Netlink messages specific to the `mac80211_hwsim` kernel module, which is used for simulating Wi-Fi hardware.
- `GenlMsgHdr`: Represents the generic Netlink message header.
- `NlAttrHdr`: Represents the Netlink attribute header.
- Attribute ID constants are defined in `mac80211_hwsim_netlink::attr_id`.

The `mac80211_hwsim_netlink_util` module includes functions for iterating over Netlink attributes, parsing common attribute types (u32, string, MAC address), building Netlink messages, and extracting 802.11 frames from `HWSIM_ATTR_FRAME_DATA` attributes.
The `mac80211_hwsim_netlink_json` module supports JSON representation of Netlink attribute headers and payloads.

## Usage

Add `packets_zc` to your `Cargo.toml` (adjust path or version as needed):
```toml
[dependencies]
packets_zc = { path = "../packets-zc" } # Example path
# Ensure zerocopy version is compatible with its usage in packets_zc
zerocopy = "0.7" # Or "0.8" or the version used by packets_zc
```

Then, use the structures to parse byte slices:
```rust
use packets_zc::ethernet::{EthernetFrame, ether_type};
use zerocopy::Ref;

fn process_ethernet_packet(data: &[u8]) {
    if let Some((ethernet_frame, payload)) = Ref::<&[u8], EthernetFrame>::from_prefix(data) {
        // Access ethernet_frame fields and payload
        match ethernet_frame.ethertype.get() {
            ether_type::IPV4 => { /* ... */ }
            _ => { /* ... */ }
        }
    }
}
```