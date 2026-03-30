// Rust definitions for statistics related structures
use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

/// Represents an invalid packet with a reason and description.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InvalidPacket {
    pub reason: String,
    pub description: String,
    // Packet content is intentionally omitted from the model to avoid large payloads in stats
}

/// The kind of radio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RadioKind {
    Unspecified,
    BluetoothLowEnergy,
    BluetoothClassic,
    BleBeacon,
    Wifi,
    Uwb,
    Nfc,
}

impl Default for RadioKind {
    fn default() -> Self {
        Self::Unspecified
    }
}

impl From<i32> for RadioKind {
    fn from(v: i32) -> Self {
        match v {
            1 => Self::BluetoothLowEnergy,
            2 => Self::BluetoothClassic,
            3 => Self::BleBeacon,
            4 => Self::Wifi,
            5 => Self::Uwb,
            6 => Self::Nfc,
            _ => Self::Unspecified,
        }
    }
}

impl From<RadioKind> for i32 {
    fn from(v: RadioKind) -> Self {
        match v {
            RadioKind::Unspecified => 0,
            RadioKind::BluetoothLowEnergy => 1,
            RadioKind::BluetoothClassic => 2,
            RadioKind::BleBeacon => 3,
            RadioKind::Wifi => 4,
            RadioKind::Uwb => 5,
            RadioKind::Nfc => 6,
        }
    }
}

/// Represents the statistics for a radio.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NetsimRadioStats {
    /// The unique ID of the chip.
    pub id: u32,
    /// The name of the chip.
    pub name: String,
    /// The kind of the radio (e.g. BLUETOOTH_LOW_ENERGY, WIFI).
    pub kind: RadioKind,
    /// Duration of the stats session in seconds.
    pub duration_secs: u64,
    /// Number of packets transmitted.
    pub tx_count: u64,
    /// Number of packets received.
    pub rx_count: u64,
    /// Number of bytes transmitted.
    pub tx_bytes: u64,
    /// Number of bytes received.
    pub rx_bytes: u64,
    /// List of invalid packets encountered (capped at 50).
    /// Use `push_invalid_packet` to add items to enforce the cap.
    invalid_packets: VecDeque<InvalidPacket>,
}

impl NetsimRadioStats {
    /// The maximum number of invalid packets to store.
    pub const MAX_INVALID_PACKETS: usize = 50;

    /// Adds an invalid packet to the stats, maintaining a maximum of 50 items.
    pub fn push_invalid_packet(&mut self, packet: InvalidPacket) {
        if self.invalid_packets.len() >= Self::MAX_INVALID_PACKETS {
            self.invalid_packets.pop_front();
        }
        self.invalid_packets.push_back(packet);
    }
}

/// Represents the statistics for a device.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NetsimDeviceStats {
    /// The unique ID of the device.
    pub device_id: Option<u32>,
    /// The name of the device.
    pub name: Option<String>,
    /// The kind of the device (e.g., "pixel6").
    pub kind: Option<String>,
    /// The version of the device.
    pub version: Option<String>,
    /// The SDK version of the device.
    pub sdk_version: Option<String>,
    /// The build ID of the device.
    pub build_id: Option<String>,
    /// The variant of the device.
    pub variant: Option<String>,
    /// The architecture of the device.
    pub arch: Option<String>,
}

/// Represents the statistics for the frontend.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct NetsimFrontendStats {
    pub get_version: u32,
    pub create_device: u32,
    pub delete_chip: u32,
    pub patch_device: u32,
    pub reset: u32,
    pub list_device: u32,
    pub subscribe_device: u32,
    pub patch_capture: u32,
    pub list_capture: u32,
    pub get_capture: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_invalid_packet_capping() {
        let mut stats = NetsimRadioStats::default();
        let overflow = 10;
        for i in 0..(NetsimRadioStats::MAX_INVALID_PACKETS + overflow) {
            stats.push_invalid_packet(InvalidPacket {
                reason: format!("reason {}", i),
                description: format!("description {}", i),
            });
        }
        assert_eq!(stats.invalid_packets.len(), NetsimRadioStats::MAX_INVALID_PACKETS);
        assert_eq!(stats.invalid_packets[0].reason, format!("reason {}", overflow));
        assert_eq!(
            stats.invalid_packets[NetsimRadioStats::MAX_INVALID_PACKETS - 1].reason,
            format!("reason {}", NetsimRadioStats::MAX_INVALID_PACKETS + overflow - 1)
        );
    }
}
