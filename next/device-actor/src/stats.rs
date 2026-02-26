use std::time::Instant;

use netsim_proto::stats::NetsimStats as ProtoNetsimStats;
use protobuf_json_mapping::print_to_string;

const DEFAULT_STATS_FILENAME: &str = "netsim_session_stats.json";

#[derive(Debug)]
pub struct Stats {
    proto: ProtoNetsimStats,
    start_time: Option<Instant>,
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

        Self { proto, start_time: Some(Instant::now()), stats_path }
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

    pub fn get_base_stats(&mut self) -> ProtoNetsimStats {
        if let Some(start) = self.start_time {
            self.proto.set_duration_secs(start.elapsed().as_secs());
        }
        self.proto.clone()
    }
}

pub(crate) fn write_combined_stats(
    base_stats: ProtoNetsimStats,
    path: std::path::PathBuf,
) -> Result<(), std::io::Error> {
    Stats::write_to_disk(&base_stats, &path)
}

impl Stats {
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
        let json = print_to_string(proto)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        use std::io::Write;
        file.write_all(json.as_bytes())?;
        file.flush()?;
        Ok(())
    }
}
