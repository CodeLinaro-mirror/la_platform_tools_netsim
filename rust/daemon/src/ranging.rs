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

//! Ranging library for RSSI.
///
/// The Free Space Path Loss (FSPL) model is considered as the standard
/// under the ideal scenario.

/// (dBm) PATH_LOSS at 1m for isotropic antenna transmitting BLE.
const PATH_LOSS_AT_1M: f32 = 40.20;

/// Convert distance to RSSI using the free space path loss equation.
/// See [Free-space_path_loss][1].
///
/// [1]: http://en.wikipedia.org/wiki/Free-space_path_loss
///
/// # Parameters
///
/// * `distance`: distance in meters (m).
/// * `tx_power`: transmitted power (dBm) calibrated to 1 meter.
///
/// # Returns
///
/// The rssi that would be measured at that distance, in the
/// range -120..20 dBm,
pub fn distance_to_rssi(tx_power: i8, distance: f32) -> i8 {
    // TODO(b/285634913)
    // Rootcanal reporting tx_power of 0 or 1 during Nearby Share
    let new_tx_power = match tx_power {
        0 | 1 => -49,
        _ => tx_power,
    };
    match distance == 0.0 {
        true => (new_tx_power as f32 + PATH_LOSS_AT_1M).clamp(-120.0, 20.0) as i8,
        false => (new_tx_power as f32 - 20.0 * distance.log10()).clamp(-120.0, 20.0) as i8,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn rssi_at_0m() {
        let rssi_at_0m = super::distance_to_rssi(-120, 0.0);
        assert_eq!(rssi_at_0m, -79);
    }

    #[test]
    fn rssi_at_1m() {
        // With transmit power at 0 dBm verify a reasonable rssi at 1m
        let rssi_at_1m = super::distance_to_rssi(0, 1.0);
        assert!(rssi_at_1m < -35 && rssi_at_1m > -55);
    }

    #[test]
    fn rssi_saturate_inf() {
        // Verify that the rssi saturates at -120 for very large distances.
        let rssi_inf = super::distance_to_rssi(-120, 1000.0);
        assert_eq!(rssi_inf, -120);
    }

    #[test]
    fn rssi_saturate_sup() {
        // Verify that the rssi saturates at +20 for the largest tx power
        // and nearest distance.
        let rssi_sup = super::distance_to_rssi(20, 0.0);
        assert_eq!(rssi_sup, 20);
    }
}
