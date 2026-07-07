// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::time::Instant;

use netsim_proto::stats::NetsimStats as ProtoNetsimStats;
use protobuf_json_mapping::{PrintOptions, print_to_string_with_options};
use tracing::warn;

const STATS_PRINT_OPTIONS: PrintOptions = PrintOptions {
    enum_values_int: false,
    proto_field_name: true,
    always_output_default_values: true,
    _future_options: (),
};

const DEFAULT_STATS_FILENAME: &str = "netsim_session_stats.json";

#[derive(Debug)]
pub struct Stats {
    proto: ProtoNetsimStats,
    start_time: Option<Instant>,
    // History of stats for deleted chips.
    // Note: This vector grows indefinitely during the session.
    // For typical usage (< 10,000 devices/session), memory overhead is negligible (~1MB).
    // For long-running stress tests, this may need a cap or disk-offload.
    archived_radio_stats: Vec<netsim_proto::stats::NetsimRadioStats>,
    pub stats_path: std::path::PathBuf,
    frontend_stats: std::sync::Arc<netsim_model::FrontendStats>,
}

impl Default for Stats {
    fn default() -> Self {
        Self::new(
            "0.0.0".to_string(),
            None,
            std::sync::Arc::new(netsim_model::FrontendStats::default()),
        )
    }
}

impl Stats {
    pub fn new(
        version: String,
        path: Option<std::path::PathBuf>,
        frontend_stats: std::sync::Arc<netsim_model::FrontendStats>,
    ) -> Self {
        let mut proto = ProtoNetsimStats::default();
        proto.set_version(version);
        proto.set_device_count(0);
        proto.set_peak_concurrent_devices(0);

        let stats_path = path.unwrap_or_else(|| {
            let mut p = common::system::netsimd_temp_dir();
            p.push(DEFAULT_STATS_FILENAME);
            p
        });

        Self {
            proto,
            start_time: Some(Instant::now()),
            archived_radio_stats: Vec::new(),
            stats_path,
            frontend_stats,
        }
    }

    pub fn update_device_count(&mut self, current_count: usize, created: bool) {
        if created {
            let new_total = self.proto.device_count() + 1;
            self.proto.set_device_count(new_total);
        }

        if (current_count as i32) > self.proto.peak_concurrent_devices() {
            self.proto.set_peak_concurrent_devices(current_count as i32);
        }
    }

    pub fn archive(&mut self, radio_stats: netsim_proto::stats::NetsimRadioStats) {
        self.archived_radio_stats.push(radio_stats);
    }

    pub fn add_device_stats(&mut self, device_stats: netsim_proto::stats::NetsimDeviceStats) {
        self.proto.device_stats.push(device_stats);
    }

    pub fn get_combined_stats(
        &mut self,
        mut active_stats: Vec<netsim_proto::stats::NetsimRadioStats>,
        wifi_stats: Option<netsim_proto::stats::WifiStats>,
        nfc_stats: Option<netsim_proto::stats::NfcStats>,
        nfc_service_stats: Option<netsim_proto::stats::NfcServiceStats>,
    ) -> ProtoNetsimStats {
        if let Some(start) = self.start_time {
            self.proto.set_duration_secs(start.elapsed().as_secs());
        }
        let mut combined = self.proto.clone();
        combined.radio_stats.extend(self.archived_radio_stats.clone());
        combined.radio_stats.append(&mut active_stats);
        if let Some(ws) = wifi_stats {
            combined.wifi_stats = Some(ws).into();
        }
        if let Some(ns) = nfc_stats {
            combined.nfc_stats = Some(ns).into();
        }
        if let Some(nss) = nfc_service_stats {
            combined.nfc_service_stats = Some(nss).into();
        }

        let frontend_snap = self.frontend_stats.snapshot();
        let mut frontend_proto = netsim_proto::stats::NetsimFrontendStats::new();
        frontend_proto.set_get_version(frontend_snap.get_version);
        frontend_proto.set_create_device(frontend_snap.create_device);
        frontend_proto.set_delete_chip(frontend_snap.delete_chip);
        frontend_proto.set_patch_device(frontend_snap.patch_device);
        frontend_proto.set_reset(frontend_snap.reset);
        frontend_proto.set_list_device(frontend_snap.list_device);
        frontend_proto.set_subscribe_device(frontend_snap.subscribe_device);
        frontend_proto.set_patch_capture(frontend_snap.patch_capture);
        frontend_proto.set_list_capture(frontend_snap.list_capture);
        frontend_proto.set_get_capture(frontend_snap.get_capture);
        frontend_proto.set_delete_device(frontend_snap.delete_device);
        combined.frontend_stats = netsim_proto::protobuf::MessageField::some(frontend_proto);

        combined
    }

    fn write_to_disk(proto: &ProtoNetsimStats, path: &std::path::Path) -> std::io::Result<()> {
        let mut tmp_path = path.to_path_buf();
        if let Some(filename) = tmp_path.file_name() {
            let mut new_filename = filename.to_os_string();
            new_filename.push(".tmp");
            tmp_path.set_file_name(new_filename);
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Invalid stats path (no filename): {:?}", path),
            ));
        }

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if let Err(e) = Self::write_json_to_file(proto, &tmp_path) {
            let _ = std::fs::remove_file(&tmp_path);
            return Err(e);
        }

        let mut rename_res = std::fs::rename(&tmp_path, path);
        if rename_res.is_err() {
            // On Windows, renaming to an existing file can fail if the destination file
            // handle is momentarily held or locked. Retry after removing destination.
            for _ in 0..5 {
                std::thread::sleep(std::time::Duration::from_millis(10));
                let _ = std::fs::remove_file(path);
                rename_res = std::fs::rename(&tmp_path, path);
                if rename_res.is_ok() {
                    break;
                }
            }
        }

        if let Err(e) = rename_res {
            warn!("Failed to replace stats file {:?} with {:?}: {}", tmp_path, path, e);
            let _ = std::fs::remove_file(&tmp_path);
            return Err(e);
        }
        Ok(())
    }

    fn write_json_to_file(proto: &ProtoNetsimStats, path: &std::path::Path) -> std::io::Result<()> {
        let mut file = std::fs::File::create(path)?;

        let json = print_to_string_with_options(proto, &STATS_PRINT_OPTIONS)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        use std::io::Write;
        file.write_all(json.as_bytes())?;
        file.flush()?;
        Ok(())
    }
}

pub(crate) fn write_combined_stats(
    stats_proto: ProtoNetsimStats,
    path: std::path::PathBuf,
) -> Result<(), std::io::Error> {
    Stats::write_to_disk(&stats_proto, &path)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, atomic::Ordering};

    use netsim_model::FrontendStats;

    use super::*;

    #[test]
    fn test_frontend_stats_translation() {
        let frontend_stats = Arc::new(FrontendStats::default());

        // Simulate API calls
        frontend_stats.get_version.store(1, Ordering::SeqCst);
        frontend_stats.create_device.store(2, Ordering::SeqCst);
        frontend_stats.delete_chip.store(3, Ordering::SeqCst);
        frontend_stats.patch_device.store(4, Ordering::SeqCst);
        frontend_stats.reset.store(5, Ordering::SeqCst);
        frontend_stats.list_device.store(6, Ordering::SeqCst);
        frontend_stats.subscribe_device.store(7, Ordering::SeqCst);
        frontend_stats.patch_capture.store(8, Ordering::SeqCst);
        frontend_stats.list_capture.store(9, Ordering::SeqCst);
        frontend_stats.get_capture.store(10, Ordering::SeqCst);
        frontend_stats.delete_device.store(11, Ordering::SeqCst);

        let mut stats = Stats::new("1.0.0".to_string(), None, frontend_stats);
        let proto = stats.get_combined_stats(vec![], None, None, None);

        let frontend_proto = proto.frontend_stats.as_ref().expect("Frontend stats missing");
        assert_eq!(frontend_proto.get_version(), 1);
        assert_eq!(frontend_proto.create_device(), 2);
        assert_eq!(frontend_proto.delete_chip(), 3);
        assert_eq!(frontend_proto.patch_device(), 4);
        assert_eq!(frontend_proto.reset(), 5);
        assert_eq!(frontend_proto.list_device(), 6);
        assert_eq!(frontend_proto.subscribe_device(), 7);
        assert_eq!(frontend_proto.patch_capture(), 8);
        assert_eq!(frontend_proto.list_capture(), 9);
        assert_eq!(frontend_proto.get_capture(), 10);
        assert_eq!(frontend_proto.delete_device(), 11);
    }
}
