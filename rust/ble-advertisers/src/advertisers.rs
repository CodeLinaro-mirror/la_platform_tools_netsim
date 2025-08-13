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

//! A thread-safe set of BLE advertisers.

use crate::{
    advertiser::Advertiser,
    link_layer::{le_scan_req_pdu::LeScanReqPdu, types::Address},
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct ManagedAdvertiser {
    advertiser: Advertiser,
    last_sent: Instant,
}

#[derive(Default)]
struct AdvertisersInner {
    advertisers: HashMap<u32, ManagedAdvertiser>,
    mac_address_to_id: HashMap<Address, u32>,
}

/// A thread-safe collection of advertisers that can be managed as a group.
///
/// This struct can be cloned and moved into multiple threads or async tasks.
#[derive(Clone, Default)]
pub struct Advertisers {
    inner: Arc<Mutex<AdvertisersInner>>,
}

impl Advertisers {
    /// Creates a new, empty advertiser set.
    pub fn new() -> Self {
        let inner =
            AdvertisersInner { advertisers: HashMap::new(), mac_address_to_id: HashMap::new() };
        Self { inner: Arc::new(Mutex::new(inner)) }
    }

    /// Adds an advertiser to the set.
    ///
    /// If the set did not have this id present, `None` is returned.
    /// If the set did have this id present, the value is updated, and the old
    /// value is returned. This operation locks the underlying mutex.
    pub fn add(&self, id: u32, advertiser: Advertiser) -> Option<Advertiser> {
        let mut inner = self.inner.lock().unwrap();
        let mac_address = advertiser.mac_address;
        let managed_advertiser = ManagedAdvertiser { advertiser, last_sent: Instant::now() };
        let old = inner.advertisers.insert(id, managed_advertiser);
        if let Some(old_adv) = &old {
            inner.mac_address_to_id.remove(&old_adv.advertiser.mac_address);
        }
        inner.mac_address_to_id.insert(mac_address, id);
        old.map(|o| o.advertiser)
    }

    /// Removes an advertiser from the set, returning the advertiser if it was present.
    /// This operation locks the underlying mutex.
    pub fn remove(&self, id: u32) -> Option<Advertiser> {
        let mut inner = self.inner.lock().unwrap();
        let removed = inner.advertisers.remove(&id);
        if let Some(managed) = &removed {
            inner.mac_address_to_id.remove(&managed.advertiser.mac_address);
        }
        removed.map(|managed| managed.advertiser)
    }

    /// Returns a clone of the advertiser's configuration with the given id.
    /// This operation locks the underlying mutex.
    pub fn get(&self, id: u32) -> Option<Advertiser> {
        let inner = self.inner.lock().unwrap();
        inner.advertisers.get(&id).map(|managed| managed.advertiser.clone())
    }

    /// Returns the advertising packet for the advertiser with the given id and
    /// updates its last sent time. This operation locks the underlying mutex.
    ///
    /// Returns `None` if the id is not present in the set.
    pub fn packet(&self, id: u32) -> Option<Vec<u8>> {
        let mut inner = self.inner.lock().unwrap();
        if let Some(managed) = inner.advertisers.get_mut(&id) {
            managed.last_sent = Instant::now();
            Some(managed.advertiser.advertising_packet().to_vec())
        } else {
            None
        }
    }

    /// Returns the scan response packet for the advertiser with the given id.
    /// This operation locks the underlying mutex.
    ///
    /// Returns `None` if the id is not present in the set or if the advertiser
    /// does not have a scan response.
    pub fn scan_response_packet(&self, id: u32) -> Option<Vec<u8>> {
        let inner = self.inner.lock().unwrap();
        inner
            .advertisers
            .get(&id)
            .and_then(|managed| managed.advertiser.scan_response_packet().map(|p| p.to_vec()))
    }

    /// Handles an incoming link-layer packet.
    ///
    /// This method will parse the packet, and if it is a scan request that
    /// targets a managed, scannable advertiser, it will return the appropriate
    /// scan response packet. Otherwise, it returns `None`.
    /// This operation locks the underlying mutex.
    pub fn handle_link_layer(&self, packet: &[u8]) -> Option<Vec<u8>> {
        let request = LeScanReqPdu::parse(packet).ok()?;
        let inner = self.inner.lock().unwrap();
        let id = inner.mac_address_to_id.get(&request.advertising_address)?;
        inner
            .advertisers
            .get(id)
            .and_then(|managed| managed.advertiser.scan_response_packet().map(|p| p.to_vec()))
    }

    /// Returns the id and duration of the next advertisement.
    /// This operation locks the underlying mutex.
    ///
    /// The duration is the time from now until the next advertisement should be sent.
    /// Returns `None` if the set is empty.
    pub fn duration(&self) -> Option<(u32, Duration)> {
        let inner = self.inner.lock().unwrap();
        let now = Instant::now();
        inner
            .advertisers
            .iter()
            .map(|(&id, managed)| {
                let elapsed = now.duration_since(managed.last_sent);
                let interval = managed.advertiser.duration();
                let remaining = interval.saturating_sub(elapsed);
                (id, remaining)
            })
            .min_by_key(|&(_, remaining)| remaining)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        advertise_data::AdvertiseData,
        advertise_settings::{AdvertiseMode, AdvertiseSettingsBuilder, TxPowerLevel},
        link_layer::types::LegacyAdvertisingType,
    };
    use std::thread;
    use zerocopy::IntoBytes;

    fn create_advertiser(interval_ms: u64, address: Address) -> Advertiser {
        let settings = AdvertiseSettingsBuilder::new()
            .mode(AdvertiseMode::new(Duration::from_millis(interval_ms)))
            .build();
        let data =
            AdvertiseData::builder("test".to_string(), TxPowerLevel::default()).build().unwrap();
        Advertiser::new(settings, data, None, address).unwrap()
    }

    #[test]
    fn test_add_remove_get() {
        let set = Advertisers::new();
        let adv1 = create_advertiser(100, [1; 6]);
        let adv2 = create_advertiser(200, [2; 6]);

        // Add
        assert!(set.add(1, adv1).is_none());
        set.add(2, adv2);
        assert_eq!(set.inner.lock().unwrap().advertisers.len(), 2);
        assert_eq!(set.inner.lock().unwrap().mac_address_to_id.len(), 2);

        // Get
        let retrieved_adv = set.get(1).unwrap();
        assert_eq!(retrieved_adv.duration(), Duration::from_millis(100));
        assert!(set.get(3).is_none());

        // Remove
        let removed = set.remove(1).unwrap();
        assert_eq!(removed.duration(), Duration::from_millis(100));
        assert_eq!(set.inner.lock().unwrap().advertisers.len(), 1);
        assert_eq!(set.inner.lock().unwrap().mac_address_to_id.len(), 1);
        assert!(set.get(1).is_none());
    }

    #[test]
    fn test_packet() {
        let set = Advertisers::new();
        let adv = create_advertiser(100, [1; 6]);
        let packet = adv.advertising_packet().to_vec();
        set.add(1, adv);

        let received_packet = set.packet(1).unwrap();
        assert_eq!(packet, received_packet);
        assert!(set.packet(2).is_none());
    }

    #[test]
    fn test_duration() {
        let set = Advertisers::new();
        let adv1 = create_advertiser(100, [1; 6]);
        let adv2 = create_advertiser(200, [2; 6]);
        set.add(1, adv1);
        set.add(2, adv2);

        thread::sleep(Duration::from_millis(10));

        let (id, duration) = set.duration().unwrap();
        assert_eq!(id, 1);
        assert!(duration <= Duration::from_millis(90));
    }

    #[test]
    fn test_duration_empty() {
        let set = Advertisers::new();
        assert!(set.duration().is_none());
    }

    #[test]
    fn test_concurrency() {
        let set = Advertisers::new();
        let set_clone = set.clone();

        let handle1 = thread::spawn(move || {
            let adv = create_advertiser(100, [1; 6]);
            set_clone.add(1, adv);
        });

        let set_clone2 = set.clone();
        let handle2 = thread::spawn(move || {
            let adv = create_advertiser(200, [2; 6]);
            set_clone2.add(2, adv);
        });

        handle1.join().unwrap();
        handle2.join().unwrap();

        assert_eq!(set.inner.lock().unwrap().advertisers.len(), 2);
        assert!(set.get(1).is_some());
        assert!(set.get(2).is_some());
    }

    #[test]
    fn test_handle_link_layer() {
        let set = Advertisers::new();
        let scannable_addr = [0xAA; 6];
        let non_scannable_addr = [0xBB; 6];

        // Create a scannable advertiser with a scan response
        let settings = AdvertiseSettingsBuilder::new().scannable().build();
        let data =
            AdvertiseData::builder("scan-me".to_string(), TxPowerLevel::default()).build().unwrap();
        let scan_response = AdvertiseData::builder("scan-rsp".to_string(), TxPowerLevel::default())
            .build()
            .unwrap();
        let scannable_adv =
            Advertiser::new(settings, data, Some(scan_response), scannable_addr).unwrap();
        let expected_response_packet = scannable_adv.scan_response_packet().unwrap().to_vec();
        set.add(1, scannable_adv);

        // Create a non-scannable advertiser
        let non_scannable_adv = create_advertiser(100, non_scannable_addr);
        set.add(2, non_scannable_adv);

        // Build a valid scan request packet targeting the scannable advertiser
        let scan_req_body =
            LeScanReqPdu { scanning_address: [0; 6], advertising_address: scannable_addr };
        let mut scan_req_bytes = vec![LegacyAdvertisingType::ScanReq as u8, 12];
        scan_req_bytes.extend_from_slice(scan_req_body.as_bytes());

        // Handle the request and check the response
        let response = set.handle_link_layer(&scan_req_bytes).unwrap();
        assert_eq!(response, expected_response_packet);

        // Test with a request to a non-scannable advertiser
        let scan_req_body_2 =
            LeScanReqPdu { advertising_address: non_scannable_addr, ..scan_req_body };
        let mut scan_req_bytes_2 = vec![LegacyAdvertisingType::ScanReq as u8, 12];
        scan_req_bytes_2.extend_from_slice(scan_req_body_2.as_bytes());
        assert!(set.handle_link_layer(&scan_req_bytes_2).is_none());

        // Test with a non-scan request packet
        let other_packet = vec![LegacyAdvertisingType::AdvInd as u8, 0, 0, 0];
        assert!(set.handle_link_layer(&other_packet).is_none());
    }
}
