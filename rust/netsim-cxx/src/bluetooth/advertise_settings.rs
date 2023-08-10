// Copyright 2023 Google LLC
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

use frontend_proto::model::chip::{
    bluetooth_beacon::advertise_settings::Advertise_mode,
    bluetooth_beacon::advertise_settings::Tx_power_level,
    bluetooth_beacon::AdvertiseSettings as AdvertiseSettingsProto,
};
use std::time::Duration;

// Default parameter value for SendLinkLayerPacket in packages/modules/Bluetooth/tools/model/devices/device.h
static DEFAULT_TX_POWER_LEVEL: i8 = 0;
// From Beacon::Beacon constructor referenced in packages/modules/Bluetooth/tools/model/devices/beacon.cc
static DEFAULT_ADVERTISE_INTERVAL: Duration = Duration::from_millis(1280);

/// Configurable settings for ble beacon advertisements.
#[derive(Debug, PartialEq)]
pub struct AdvertiseSettings {
    /// Time interval between advertisements.
    pub mode: AdvertiseMode,
    /// Transmit power level for advertisements and scan responses.
    pub tx_power_level: TxPowerLevel,
    /// Whether the beacon will respond to scan requests.
    pub scannable: bool,
    /// How long to send advertisements for before stopping.
    pub timeout: Option<Duration>,
}

impl AdvertiseSettings {
    /// Returns a new advertise settings builder with no fields.
    pub fn builder() -> Builder {
        Builder::default()
    }

    pub fn from_proto(proto: &AdvertiseSettingsProto) -> Result<Builder, String> {
        let mut builder = Builder::default();

        if let Some(mode) = proto.advertise_mode.as_ref() {
            builder.mode(mode.into());
        }

        if let Some(tx_power) = proto.tx_power_level.as_ref() {
            builder.tx_power_level(tx_power.try_into()?);
        }

        if proto.scannable {
            builder.scannable();
        }

        if proto.timeout != u64::default() {
            builder.timeout(Duration::from_millis(proto.timeout));
        }

        Ok(builder)
    }
}

impl TryFrom<&AdvertiseSettings> for AdvertiseSettingsProto {
    type Error = String;

    fn try_from(value: &AdvertiseSettings) -> Result<Self, Self::Error> {
        Ok(AdvertiseSettingsProto {
            advertise_mode: Some(value.mode.try_into()?),
            tx_power_level: Some(value.tx_power_level.into()),
            scannable: value.scannable,
            timeout: value.timeout.unwrap_or_default().as_millis().try_into().map_err(|_| {
                String::from("could not convert timeout to millis: must fit in a u64")
            })?,
            ..Default::default()
        })
    }
}

#[derive(Default)]
/// Builder for BLE beacon advertise settings.
pub struct Builder {
    mode: Option<AdvertiseMode>,
    tx_power_level: Option<TxPowerLevel>,
    scannable: bool,
    timeout: Option<Duration>,
}

impl Builder {
    /// Set the advertise mode.
    pub fn mode(&mut self, mode: AdvertiseMode) -> &mut Self {
        self.mode = Some(mode);
        self
    }

    /// Set the transmit power level.
    pub fn tx_power_level(&mut self, tx_power_level: TxPowerLevel) -> &mut Self {
        self.tx_power_level = Some(tx_power_level);
        self
    }

    /// Set whether the beacon will respond to scan requests.
    pub fn scannable(&mut self) -> &mut Self {
        self.scannable = true;
        self
    }

    /// Set how long the beacon will send advertisements for.
    pub fn timeout(&mut self, timeout: Duration) -> &mut Self {
        self.timeout = Some(timeout);
        self
    }

    /// Build the advertise settings.
    pub fn build(&mut self) -> AdvertiseSettings {
        AdvertiseSettings {
            mode: self.mode.unwrap_or_default(),
            tx_power_level: self.tx_power_level.unwrap_or_default(),
            scannable: self.scannable,
            timeout: self.timeout,
        }
    }
}

/// A ble beacon advertise mode. Can be casted to/from a(n):
/// * `std::time::Duration` representing the time interval between advertisements
/// * `model::chip::bluetooth_beacon::advertise_settings::Advertise_mode`
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct AdvertiseMode {
    interval: Duration,
}

impl AdvertiseMode {
    pub fn get_interval(&self) -> Duration {
        self.interval
    }
}

impl Default for AdvertiseMode {
    fn default() -> Self {
        Self { interval: DEFAULT_ADVERTISE_INTERVAL }
    }
}

impl From<&Advertise_mode> for AdvertiseMode {
    fn from(value: &Advertise_mode) -> Self {
        match value {
            Advertise_mode::ModeNumeric(interval) => {
                Self { interval: Duration::from_millis(*interval) }
            }
            // TODO(jmes): Support named advertising modes b/294260722
            _ => todo!("named advertising modes are not yet implemented"),
        }
    }
}

impl TryFrom<AdvertiseMode> for Advertise_mode {
    type Error = String;

    fn try_from(value: AdvertiseMode) -> Result<Self, Self::Error> {
        Ok(Advertise_mode::ModeNumeric(value.interval.as_millis().try_into().map_err(|_| {
            String::from(
                "failed to convert duration to AdvertiseMode: number of milliseconds was larger than a u64",
            )
        })?))
    }
}

impl From<AdvertiseMode> for Duration {
    fn from(value: AdvertiseMode) -> Self {
        value.interval
    }
}

impl From<Duration> for AdvertiseMode {
    fn from(value: Duration) -> Self {
        Self { interval: value }
    }
}

/// A ble beacon transmit power level. Can be casted to/from a(n):
/// * `i8` measuring power in dBm
/// * `model::chip::bluetooth_beacon::advertise_settings::Tx_power_level`
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct TxPowerLevel {
    tx_power: i8,
}

impl Default for TxPowerLevel {
    fn default() -> Self {
        TxPowerLevel { tx_power: DEFAULT_TX_POWER_LEVEL }
    }
}

impl TryFrom<&Tx_power_level> for TxPowerLevel {
    type Error = String;

    fn try_from(value: &Tx_power_level) -> Result<Self, Self::Error> {
        Ok(match value {
            Tx_power_level::LevelNumeric(tx_power) => Self {
                tx_power: (*tx_power)
                    .try_into()
                    .map_err(|_| "tx power level was too large: it must fit in an i8")?,
            },
            // TODO(jmes): Support named tx power levels b/294260722
            _ => todo!("named tx power levels are not yet implemented"),
        })
    }
}

impl From<TxPowerLevel> for Tx_power_level {
    fn from(value: TxPowerLevel) -> Self {
        Tx_power_level::LevelNumeric(value.tx_power.into())
    }
}

impl From<TxPowerLevel> for i8 {
    fn from(value: TxPowerLevel) -> Self {
        value.tx_power
    }
}

impl From<i8> for TxPowerLevel {
    fn from(value: i8) -> Self {
        Self { tx_power: value }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build() {
        let mode: AdvertiseMode = Duration::from_millis(200).into();
        let tx_power_level: TxPowerLevel = (-1).into();
        let timeout = Duration::from_millis(8000);

        let settings = AdvertiseSettings::builder()
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
    fn test_from_proto_succeeds() {
        let mode = Advertise_mode::ModeNumeric(150);
        let tx_power_level = Tx_power_level::LevelNumeric(3);
        let timeout_ms = 5555;

        let proto = AdvertiseSettingsProto {
            advertise_mode: Some(mode.clone()),
            tx_power_level: Some(tx_power_level.clone()),
            scannable: true,
            timeout: timeout_ms,
            ..Default::default()
        };

        let settings = AdvertiseSettings::from_proto(&proto);
        assert!(settings.is_ok());

        let tx_power_level: Result<TxPowerLevel, _> = (&tx_power_level).try_into();
        assert!(tx_power_level.is_ok());
        let tx_power_level = tx_power_level.unwrap();

        let exp_settings = AdvertiseSettings::builder()
            .mode((&mode).into())
            .tx_power_level(tx_power_level)
            .scannable()
            .timeout(Duration::from_millis(timeout_ms))
            .build();

        assert_eq!(exp_settings, settings.unwrap().build());
    }

    #[test]
    fn test_from_proto_fails() {
        let proto = AdvertiseSettingsProto {
            tx_power_level: Some(Tx_power_level::LevelNumeric((std::i8::MAX as i32) + 1)),
            ..Default::default()
        };

        assert!(AdvertiseSettings::from_proto(&proto).is_err());
    }

    #[test]
    fn test_into_proto() {
        let proto = AdvertiseSettingsProto {
            advertise_mode: Some(Advertise_mode::ModeNumeric(123)),
            tx_power_level: Some(Tx_power_level::LevelNumeric(-3)),
            scannable: true,
            timeout: 1234,
            ..Default::default()
        };

        let settings = AdvertiseSettings::from_proto(&proto);
        assert!(settings.is_ok());
        let settings: Result<AdvertiseSettingsProto, _> = (&settings.unwrap().build()).try_into();
        assert!(settings.is_ok());

        assert_eq!(proto, settings.unwrap());
    }

    #[test]
    fn test_from_proto_timeout_unset() {
        let proto = AdvertiseSettingsProto { ..Default::default() };

        let settings = AdvertiseSettings::from_proto(&proto);
        assert!(settings.is_ok());
        let settings = settings.unwrap();

        assert!(settings.timeout.is_none());
    }
}
