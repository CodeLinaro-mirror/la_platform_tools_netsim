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

//! A module to collect and write session stats

use crate::events::Event;
use anyhow::Context;
use log::info;
use netsim_common::system::netsimd_temp_dir;
use netsim_proto::stats::NetsimStats;
use protobuf_json_mapping::print_to_string;
use std::fs::File;
use std::io::Write;
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread::{Builder, JoinHandle};
use std::time::Instant;

pub struct Session {
    // Handle for the session monitor thread
    handle: Option<JoinHandle<String>>,
    // Session info is accessed by multiple threads
    info: Arc<RwLock<SessionInfo>>,
}

// Fields accessed by threads
struct SessionInfo {
    stats_proto: NetsimStats,
    current_device_count: i32,
    session_start: Instant,
}

impl Session {
    pub fn new() -> Self {
        Session {
            handle: None,
            info: Arc::new(RwLock::new(SessionInfo {
                stats_proto: NetsimStats::new(),
                current_device_count: 0,
                session_start: Instant::now(),
            })),
        }
    }

    pub fn start(&mut self, events_rx: Receiver<Event>) -> &mut Self {
        let info = self.info.clone();
        self.handle = Some(
            Builder::new()
                .name("session_monitor".to_string())
                .spawn(move || {
                    loop {
                        let event = match events_rx.recv() {
                            Ok(event) => event,
                            Err(_) => return "event receiver unexpectedly closed".to_string(),
                        };
                        // Hold the write lock for the duration of this loop iteration
                        let mut lock = info.write().expect("Could not acquire session lock");
                        match event {
                            Event::ShutDown { reason } => {
                                // Shutting down, save the session duration and exit
                                let duration_secs = lock.session_start.elapsed().as_secs();
                                lock.stats_proto.set_duration_secs(duration_secs);
                                return reason;
                            }

                            Event::DeviceRemoved { .. } => {
                                lock.current_device_count -= 1;
                            }

                            Event::DeviceAdded { .. } => {
                                // update the current_device_count and peak device usage
                                lock.current_device_count += 1;
                                let current_device_count = lock.current_device_count;
                                // incremement total number of devices started
                                let device_count = lock.stats_proto.device_count();
                                lock.stats_proto.set_device_count(device_count + 1);
                                // track the peak device usage
                                if current_device_count > lock.stats_proto.peak_concurrent_devices()
                                {
                                    lock.stats_proto
                                        .set_peak_concurrent_devices(current_device_count);
                                }
                            }
                            Event::ChipRemoved { radio_stats, device_id, .. } => {
                                // Update the radio stats proto when a
                                // chip is removed.  In the case of
                                // bluetooth there will be 2 radios,
                                // otherwise 1
                                for mut r in radio_stats {
                                    r.set_device_id(device_id);
                                    lock.stats_proto.radio_stats.push(r);
                                }
                            }
                            _ => {
                                // other events are ignored
                            }
                        }
                    }
                })
                .expect("failed to start session monitor thread"),
        );
        self
    }

    // Writes the session stats file.
    //
    // Waits for the session monitor thread to finish and writes
    // the session proto to a json file. Consumes the session.
    pub fn stop(mut self) -> anyhow::Result<()> {
        if !self.handle.as_ref().expect("no session monitor").is_finished() {
            info!("session monitor active, waiting...");
        }
        // Synchronize on session monitor thread
        self.handle.take().map(JoinHandle::join);
        let filename = netsimd_temp_dir().join("session_stats.json");
        info!("session stats to {}", filename.display());
        // session monitor thread is now shutdown...  write out the protobuf
        let mut file = File::create(filename)?;
        let lock = self.info.read().expect("Could not acquire session lock");
        let json = print_to_string(&lock.stats_proto)?;
        file.write(json.as_bytes()).context("Unable to write json session stats")?;
        file.flush()?;
        Ok(())
    }
}
