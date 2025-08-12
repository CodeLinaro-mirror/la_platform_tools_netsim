// Copyright 2025 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Configuration for an advertiser's behavior, such as interval and power.

use crate::link_layer::types::LegacyAdvertisingType;
use std::time::Duration;

// From packages/modules/Bluetooth/framework/java/android/bluetooth/le/BluetoothLeAdvertiser.java#151
const MODE_LOW_POWER_MS: u64 = 1000;

// From packages/modules/Bluetooth/framework/java/android/bluetooth/le/BluetoothLeAdvertiser.java#159
const TX_POWER_LOW_DBM: i8 = -15;

/// Configurable settings for a BLE beacon's advertisements.
///
/// Use the `builder()` to construct this struct. Any fields that are not
/// explicitly set will use their default values.
#[derive(Debug, PartialEq, Clone)]
pub struct AdvertiseSettings {
    /// Time interval between advertisements.
    pub mode: AdvertiseMode,
    /// Transmit power level for advertisements.
    pub tx_power_level: TxPowerLevel,
    /// Whether the beacon will respond to scan requests.
    pub scannable: bool,
    /// How long to send advertisements for before stopping. A value of `None`
    /// means the advertising will continue indefinitely.
    pub timeout: Option<Duration>,
}

impl AdvertiseSettings {
    /// Returns a new builder for creating `AdvertiseSettings`.
    pub fn builder() -> AdvertiseSettingsBuilder {
        AdvertiseSettingsBuilder::default()
    }

    /// Returns the PDU type of advertise packets with the provided settings.
    pub fn get_packet_type(&self) -> LegacyAdvertisingType {
        if self.scannable {
            LegacyAdvertisingType::AdvScanInd
        } else {
            LegacyAdvertisingType::AdvNonconnInd
        }
    }
}

/// A builder for creating `AdvertiseSettings`.
#[derive(Default)]
pub struct AdvertiseSettingsBuilder {
    mode: Option<AdvertiseMode>,
    tx_power_level: Option<TxPowerLevel>,
    scannable: bool,
    timeout: Option<Duration>,
}

impl AdvertiseSettingsBuilder {
    /// Returns a new, default advertise settings builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the `AdvertiseSettings`.
    pub fn build(&self) -> AdvertiseSettings {
        AdvertiseSettings {
            mode: self.mode.unwrap_or_default(),
            tx_power_level: self.tx_power_level.unwrap_or_default(),
            scannable: self.scannable,
            timeout: self.timeout,
        }
    }

    /// Sets the advertising interval mode.
    pub fn mode(&mut self, mode: AdvertiseMode) -> &mut Self {
        self.mode = Some(mode);
        self
    }

    /// Sets the transmit power level.
    pub fn tx_power_level(&mut self, tx_power_level: TxPowerLevel) -> &mut Self {
        self.tx_power_level = Some(tx_power_level);
        self
    }

    /// Sets the advertiser to be scannable, meaning it will respond to scan
    /// requests.
    pub fn scannable(&mut self) -> &mut Self {
        self.scannable = true;
        self
    }

    /// Sets a timeout for how long the advertiser will send packets.
    pub fn timeout(&mut self, timeout: Duration) -> &mut Self {
        self.timeout = Some(timeout);
        self
    }
}

/// A BLE beacon advertising interval mode.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct AdvertiseMode {
    /// The time interval between advertisements.
    pub interval: Duration,
}

impl AdvertiseMode {
    /// Creates a new `AdvertiseMode` from a specific `Duration`.
    pub fn new(interval: Duration) -> Self {
        AdvertiseMode { interval }
    }
}

impl Default for AdvertiseMode {
    fn default() -> Self {
        Self { interval: Duration::from_millis(MODE_LOW_POWER_MS) }
    }
}

/// A BLE beacon transmit power level.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TxPowerLevel {
    /// The transmit power in dBm.
    pub dbm: i8,
}

impl TxPowerLevel {
    /// Creates a new `TxPowerLevel` from an `i8` measuring power in dBm.
    pub fn new(dbm: i8) -> Self {
        TxPowerLevel { dbm }
    }
}

impl Default for TxPowerLevel {
    fn default() -> Self {
        TxPowerLevel { dbm: TX_POWER_LOW_DBM }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        let mode = AdvertiseMode::new(Duration::from_millis(200));
        let tx_power_level = TxPowerLevel::new(-1);
        let timeout = Duration::from_millis(8000);

        let settings = AdvertiseSettingsBuilder::new()
            .mode(mode)
            .tx_power_level(tx_power_level)
            .scannable()
            .timeout(timeout)
            .build();

        assert_eq!(
            AdvertiseSettings { mode, tx_power_level, scannable: true, timeout: Some(timeout) },
            settings
        )
    }

    #[test]
    fn test_default_build() {
        let settings = AdvertiseSettingsBuilder::new().build();
        assert_eq!(
            AdvertiseSettings {
                mode: AdvertiseMode::default(),
                tx_power_level: TxPowerLevel::default(),
                scannable: false,
                timeout: None
            },
            settings
        )
    }
}
