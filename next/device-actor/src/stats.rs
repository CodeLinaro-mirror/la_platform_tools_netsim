use std::time::Instant;

use netsim_proto::stats::NetsimStats as ProtoNetsimStats;
use protobuf_json_mapping::{print_to_string_with_options, PrintOptions};

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
}

impl Default for Stats {
    fn default() -> Self {
        Self::new("0.0.0".to_string(), None)
    }
}

impl Stats {
    pub fn new(version: String, path: Option<std::path::PathBuf>) -> Self {
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

        if let Err(e) = std::fs::rename(&tmp_path, path) {
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
