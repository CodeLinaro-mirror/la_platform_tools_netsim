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

//! The internal structure of CaptureInfo and CaptureMaps
//!
//! CaptureInfo is the internal structure of any Capture that includes
//! the protobuf structure. CaptureMaps contains mappings of ChipId
//! and FacadeId to CaptureInfo.

use std::collections::btree_map::{Iter, Values};
use std::collections::{BTreeMap, HashMap};
use std::fs::{File, OpenOptions};
use std::io::Result;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use frontend_proto::{
    common::ChipKind,
    model::{Capture as ProtoCapture, State},
};
use protobuf::well_known_types::timestamp::Timestamp;

use crate::ffi::get_facade_id;
use crate::system;

use super::pcap_util::write_pcap_header;

/// ChipId: i32 (for distinguishing identifiers)
pub type ChipId = i32;
/// FacadeId: i32 (for distinguishing identifiers)
pub type FacadeId = i32;

/// Internal Capture struct
pub struct CaptureInfo {
    facade_id: FacadeId,
    /// Some(File) if the file is opened and capture is actively happening.
    /// None if the file is not opened.
    pub file: Option<File>,
    // Following items will be returned as ProtoCapture. (state: file.is_some())
    id: ChipId,
    /// ChipKind (BLUETOOTH, WIFI, or UWB)
    pub chip_kind: ChipKind,
    /// Device name
    pub device_name: String,
    /// Size of pcap file
    pub size: usize,
    /// Number of packet records
    pub records: i32,
    /// Timestamp as seconds
    pub seconds: i64,
    /// Timestamp as sub-nanoseconds
    pub nanos: i32,
    /// Boolean status of whether the device is connected to netsim
    pub valid: bool,
}

/// Captures contains a recent copy of all chips and their ChipKind, chip_id,
/// and owning device name.
///
/// Information for any recent or ongoing captures is also stored in the ProtoCapture.
/// facade_key_to_capture allows for fast lookups when handle_request, handle_response
/// is invoked from packet_hub.
pub struct Captures {
    /// A mapping of (chip kind, facade id) to CaptureInfo.
    pub facade_key_to_capture: HashMap<(ChipKind, FacadeId), Arc<Mutex<CaptureInfo>>>,
    /// A mapping of chip id to CaptureInfo.
    ///
    /// BTreeMap is used for chip_id_to_capture, so that the CaptureInfo can always be
    /// ordered by ChipId. ListCaptureResponse will produce a ordered list of CaptureInfos.
    pub chip_id_to_capture: BTreeMap<ChipId, Arc<Mutex<CaptureInfo>>>,
}

impl CaptureInfo {
    /// Create an instance of CaptureInfo
    pub fn new(chip_kind: ChipKind, chip_id: ChipId, device_name: String) -> Self {
        CaptureInfo {
            facade_id: get_facade_id(chip_id),
            id: chip_id,
            chip_kind,
            device_name,
            size: 0,
            records: 0,
            seconds: 0,
            nanos: 0,
            valid: true,
            file: None,
        }
    }

    /// Creates a pcap file with headers and store it under temp directory.
    ///
    /// The lifecycle of the file is NOT tied to the lifecycle of the struct
    /// Format: /tmp/netsimd/$USER/pcaps/{chip_id}-{device_name}-{chip_kind}.pcap
    pub fn start_capture(&mut self) -> Result<()> {
        if self.file.is_some() {
            return Ok(());
        }
        let mut filename = system::netsimd_temp_dir();
        filename.push("pcaps");
        std::fs::create_dir_all(&filename)?;
        filename.push(format!("{:?}-{:}-{:?}.pcap", self.id, self.device_name, self.chip_kind));
        let mut file = OpenOptions::new().write(true).truncate(true).create(true).open(filename)?;
        let size = write_pcap_header(&mut file)?;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
        self.size = size;
        self.records = 0;
        self.seconds = timestamp.as_secs() as i64;
        self.nanos = timestamp.subsec_nanos() as i32;
        self.file = Some(file);
        Ok(())
    }

    /// Closes file by removing ownership of self.file.
    ///
    /// Capture info will still retain the size and record count
    /// So it can be downloaded easily when GetCapture is invoked.
    pub fn stop_capture(&mut self) {
        self.file = None;
    }

    /// A static helper function that returns facade_key from chip kind and facade id.
    ///
    /// This key will be used for facade_key_to_capture.
    pub fn new_facade_key(kind: ChipKind, facade_id: FacadeId) -> (ChipKind, FacadeId) {
        (kind, facade_id)
    }

    fn get_facade_key(&self) -> (ChipKind, FacadeId) {
        CaptureInfo::new_facade_key(self.chip_kind, self.facade_id)
    }

    /// Returns a Capture protobuf from CaptureInfo
    pub fn get_capture_proto(&self) -> ProtoCapture {
        let timestamp =
            Timestamp { seconds: self.seconds, nanos: self.nanos, ..Default::default() };
        ProtoCapture {
            id: self.id,
            chip_kind: self.chip_kind.into(),
            device_name: self.device_name.clone(),
            state: match self.file.is_some() {
                true => State::ON.into(),
                false => State::OFF.into(),
            },
            size: self.size as i32,
            records: self.records,
            timestamp: Some(timestamp).into(),
            valid: self.valid,
            ..Default::default()
        }
    }
}

impl Captures {
    /// Create an instance of Captures, which includes 2 empty hashmaps
    pub fn new() -> Self {
        Captures {
            facade_key_to_capture: HashMap::<(ChipKind, FacadeId), Arc<Mutex<CaptureInfo>>>::new(),
            chip_id_to_capture: BTreeMap::<ChipId, Arc<Mutex<CaptureInfo>>>::new(),
        }
    }

    /// Returns true if key exists in Captures.chip_id_to_capture
    pub fn contains(&self, key: ChipId) -> bool {
        self.chip_id_to_capture.contains_key(&key)
    }

    /// Returns an Option of lockable and mutable CaptureInfo with given key
    pub fn get(&mut self, key: ChipId) -> Option<&mut Arc<Mutex<CaptureInfo>>> {
        self.chip_id_to_capture.get_mut(&key)
    }

    /// Inserts the given CatpureInfo into Captures hashmaps
    pub fn insert(&mut self, capture: CaptureInfo) {
        let chip_id = capture.id;
        let facade_key = capture.get_facade_key();
        let arc_capture = Arc::new(Mutex::new(capture));
        self.chip_id_to_capture.insert(chip_id, arc_capture.clone());
        self.facade_key_to_capture.insert(facade_key, arc_capture);
    }

    /// Returns true if chip_id_to_capture is empty
    pub fn is_empty(&self) -> bool {
        self.chip_id_to_capture.is_empty()
    }

    /// Returns an iterable object of chip_id_to_capture hashmap
    pub fn iter(&self) -> Iter<ChipId, Arc<Mutex<CaptureInfo>>> {
        self.chip_id_to_capture.iter()
    }

    /// Removes a CaptureInfo with given key from Captures
    ///
    /// When Capture is removed, remove from each map and also invoke closing of files.
    pub fn remove(&mut self, key: &ChipId) {
        if let Some(arc_capture) = self.chip_id_to_capture.get(key) {
            if let Ok(mut capture) = arc_capture.lock() {
                self.facade_key_to_capture.remove(&capture.get_facade_key());
                capture.stop_capture();
            }
        } else {
            println!("key does not exist in Captures");
            return;
        }
        self.chip_id_to_capture.remove(key);
    }

    /// Returns Values of chip_id_to_capture hashmap values
    pub fn values(&self) -> Values<ChipId, Arc<Mutex<CaptureInfo>>> {
        self.chip_id_to_capture.values()
    }
}

impl Default for Captures {
    fn default() -> Self {
        Self::new()
    }
}
