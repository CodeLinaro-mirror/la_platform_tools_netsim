// Copyright 2026 The Android Open Source Project

//! PCAP Writer utilities for Bluetooth Sniffer

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

// --- PCAP Pseudo-Header Flags ---
const BB_FLAG_BR_EDR: u16 = 0x0001;
const BB_FLAG_DECRYPTED: u16 = 0x0008;
const BB_FLAG_HAVE_LAP: u16 = 0x0010;
const BB_FLAG_HAVE_UAP: u16 = 0x0020;
const BB_FLAG_HAVE_HEC: u16 = 0x0080;
const BB_FLAG_HAVE_P_HEADER: u16 = 0x0400;
const BB_FLAG_PAYLOAD_DATA: u16 = 0x0800;

const DEFAULT_BB_FLAGS: u16 = BB_FLAG_BR_EDR
    | BB_FLAG_DECRYPTED
    | BB_FLAG_HAVE_LAP
    | BB_FLAG_HAVE_UAP
    | BB_FLAG_HAVE_HEC
    | BB_FLAG_HAVE_P_HEADER
    | BB_FLAG_PAYLOAD_DATA;

// --- Rootcanal / Packet Types ---
const RC_PACKET_TYPE_ACL: u8 = 0x01;
const RC_PACKET_TYPE_PAGE: u8 = 0x10;
const BB_PTYPE_FHS: u8 = 0b0010;
const BB_PTYPE_DM1: u8 = 0b0011;
const BB_PTYPE_DH5: u8 = 0b1111; // Used for multi-slot packets containing > 17 bytes

// --- Control / L2CAP Fields ---
const LLID_L2CAP_START: u8 = 2;
const FLOW_GO: u8 = 1;

// --- BLE Constants ---
const LE_ADV_ACCESS_ADDRESS: u32 = 0x8E89BED6;
const LE_LEGACY_ADVERTISING_PDU: u8 = 0x0B;

#[repr(C, packed)]
#[derive(Debug, Copy, Clone, FromBytes, IntoBytes, Immutable, KnownLayout, Unaligned)]
pub struct BredrBbFrameHeader {
    pub rf_channel: u8,
    pub signal_power: u8,
    pub noise_power: u8,
    pub access_code_offenses: u8,
    pub rate_transport: u8,
    pub corrected_header_bits: u8,
    pub corrected_payload_bits: u16,
    pub lap: u32,
    pub ref_lap: [u8; 3],
    pub ref_uap: u8,
    pub packet_header: u32,
    pub flags: u16,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone, FromBytes, IntoBytes, Immutable, KnownLayout, Unaligned)]
pub struct LeLlFrameHeader {
    pub access_address: u32,
    pub pdu_header: [u8; 2],
}

/// Reverses the bits in a byte. Used for Bluetooth HEC/CRC calculations
/// where the spec often defines shift registers operating on LSB first.
fn reverse_byte(b: u8) -> u8 {
    let lookup: [u8; 16] = [
        0b0000, 0b1000, 0b0100, 0b1100, 0b0010, 0b1010, 0b0110, 0b1110, 0b0001, 0b1001, 0b0101,
        0b1101, 0b0011, 0b1011, 0b0111, 0b1111,
    ];
    (lookup[(b & 0xF) as usize] << 4) | lookup[(b >> 4) as usize]
}

/// Computes the 8-bit Header Error Check (HEC) for Bluetooth Baseband packets.
/// The HEC LFSR is initialized with the Upper Address Part (UAP) of the master.
/// (Reference: Bluetooth Core Spec v5.0, Vol 2, Part B, Section 3.1.1)
fn header_error_check(uap: u8, mut data: u32) -> u8 {
    let mut value = reverse_byte(uap);
    for _ in 0..10 {
        let bit = ((value ^ (data as u8)) & 1) != 0;
        data >>= 1;
        value >>= 1;
        if bit {
            value ^= 0xe5;
        }
    }
    value
}

/// A simple bit-level writer to construct packed bitfields across byte
/// boundaries.
struct BitWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> BitWriter<'a> {
    fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    fn write(&mut self, val: u32, bits: usize) {
        for i in 0..bits {
            let bit = (val >> i) & 1;
            let byte_idx = self.pos / 8;
            let bit_idx = self.pos % 8;
            self.buf[byte_idx] |= (bit as u8) << bit_idx;
            self.pos += 1;
        }
    }
}

/// Constructs the 18-bit Bluetooth packet header and appends the 8-bit HEC.
/// The header contains: LT_ADDR (3 bits), Type (4 bits), Flow (1 bit),
/// ARQN (1 bit), SEQN (1 bit), and HEC (8 bits).
fn build_bt_packet_header(
    uap: u8,
    lt_addr: u8,
    packet_type: u8,
    flow: bool,
    arqn: bool,
    seqn: bool,
) -> u32 {
    let mut header: u32 = (lt_addr as u32 & 0x7)
        | (((packet_type as u32) & 0xF) << 3)
        | ((flow as u32) << 7)
        | ((arqn as u32) << 8)
        | ((seqn as u32) << 9);
    header |= (header_error_check(uap, header) as u32) << 10;
    header
}

pub fn create_bredr_bb_packet(packet: &[u8]) -> Option<Vec<u8>> {
    if packet.len() < 7 {
        return None;
    }

    let p_type = packet[0];
    let is_page = match p_type {
        RC_PACKET_TYPE_PAGE => true,
        RC_PACKET_TYPE_ACL => false,
        _ => return None,
    };

    if is_page && packet.len() < 10 {
        return None;
    }

    let address = &packet[1..7];
    let lap = (address[0] as u32) | ((address[1] as u32) << 8) | ((address[2] as u32) << 16);
    let uap = address[3];
    let nap = (address[4] as u16) | ((address[5] as u16) << 8);

    let payload = if is_page { &[] } else { &packet[7..] };
    let packet_type_bits = if is_page {
        BB_PTYPE_FHS
    } else if payload.len() <= 17 {
        BB_PTYPE_DM1 // 1-byte payload header fits up to 17 bytes
    } else {
        BB_PTYPE_DH5 // 2-byte payload header fits up to 1021 bytes
    };

    let header = BredrBbFrameHeader {
        rf_channel: 0,
        signal_power: 0,
        noise_power: 0,
        access_code_offenses: 0,
        rate_transport: 0x30, // 1 Mbps Basic Rate
        corrected_header_bits: 0,
        corrected_payload_bits: 0,
        lap,
        ref_lap: [address[0], address[1], address[2]],
        ref_uap: uap,
        packet_header: build_bt_packet_header(uap, 0, packet_type_bits, true, true, true),
        flags: DEFAULT_BB_FLAGS,
    };

    let mut bb = Vec::with_capacity(64);
    bb.extend_from_slice(header.as_bytes());

    if is_page {
        bb.extend_from_slice(&0u32.to_le_bytes()); // parity bits
        let class_of_device = &packet[7..10];

        // 18-byte packed FHS (Frequency Hop Synchronization) payload
        // Structure is highly bit-packed as defined in BT Core Spec Vol 2, Part B,
        // 6.5.1
        let mut fhs = [0u8; 18];
        let mut w = BitWriter::new(&mut fhs);

        w.write(0, 32); // Parity bits
        w.write(lap, 24); // LAP
        w.write(0, 4); // Undefined / SR / SP
        w.write(uap as u32, 8); // UAP
        w.write(nap as u32, 16); // NAP
        w.write(class_of_device[0] as u32, 8); // Class of Device
        w.write(class_of_device[1] as u32, 8);
        w.write(class_of_device[2] as u32, 8);
        w.write(1, 3); // LT_ADDR
        w.write(0, 26 + 3); // Clock and Page Scan Mode

        bb.extend_from_slice(&fhs);
    } else {
        let len = payload.len();
        if len <= 17 {
            // 1-byte payload header (5-bit length)
            bb.push(LLID_L2CAP_START | (FLOW_GO << 2) | ((len as u8) << 3));
        } else {
            // 2-byte payload header (9-bit length)
            let p_header: u16 =
                (LLID_L2CAP_START as u16) | ((FLOW_GO as u16) << 2) | ((len as u16 & 0x1FF) << 3);
            bb.extend_from_slice(&p_header.to_le_bytes());
        }
        bb.extend_from_slice(payload);
        bb.extend_from_slice(&0u16.to_le_bytes()); // CRC
    }

    Some(bb)
}

/// Packages a raw Link Layer packet into a `LINKTYPE_BLUETOOTH_LE_LL` PCAP
/// format. The LE_LL pseudo-header requires the Access Address and a 2-byte PDU
/// header.
pub fn create_le_ll_packet(packet: &[u8]) -> Option<Vec<u8>> {
    if packet.len() < 16 || packet[0] != LE_LEGACY_ADVERTISING_PDU {
        return None;
    }

    let source_address = &packet[1..7];
    let tx_add = packet[13] & 1;
    let rx_add = packet[14] & 1;
    let adv_type = packet[15] & 0x0F;
    let adv_data = &packet[16..];

    let header = LeLlFrameHeader {
        access_address: LE_ADV_ACCESS_ADDRESS,
        pdu_header: [adv_type | (tx_add << 6) | (rx_add << 7), (6 + adv_data.len()) as u8],
    };

    let mut bb = Vec::with_capacity(32);
    bb.extend_from_slice(header.as_bytes());
    bb.extend_from_slice(source_address);
    bb.extend_from_slice(adv_data);
    bb.extend_from_slice(&[0, 0, 0]); // CRC

    Some(bb)
}
