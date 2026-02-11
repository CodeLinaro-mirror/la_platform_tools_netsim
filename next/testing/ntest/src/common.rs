use std::{fmt, str::FromStr, time::Instant};

pub const BUFFER_SIZE_TCP: usize = 64 * 1024;
pub const BUFFER_SIZE_UDP: usize = 65535;
pub const PAYLOAD_BYTE: u8 = 0xAB;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Tcp,
    Udp,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::Tcp => write!(f, "tcp"),
            Protocol::Udp => write!(f, "udp"),
        }
    }
}

impl FromStr for Protocol {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "tcp" => Ok(Protocol::Tcp),
            "udp" => Ok(Protocol::Udp),
            _ => Err(anyhow::anyhow!("Unknown protocol: {}", s)),
        }
    }
}

pub struct ThroughputMonitor {
    start_interval: Instant,
    bytes_in_interval: usize,
    start_total: Instant,
    total_bytes: usize,
}

impl ThroughputMonitor {
    pub fn new() -> Self {
        let now = Instant::now();
        Self { start_interval: now, bytes_in_interval: 0, start_total: now, total_bytes: 0 }
    }

    pub fn update(&mut self, bytes: usize) {
        self.bytes_in_interval += bytes;
        self.total_bytes += bytes;

        let now = Instant::now();
        let elapsed = now.duration_since(self.start_interval).as_secs_f64();

        if elapsed >= 1.0 {
            self.print_stats(elapsed, self.bytes_in_interval);
            self.start_interval = now;
            self.bytes_in_interval = 0;
        }
    }

    pub fn print_final_stats(&self) {
        let now = Instant::now();
        let total_elapsed = now.duration_since(self.start_total).as_secs_f64();
        eprintln!("- - - - - - - - - - - - - - - - - - - - - - - - -");
        self.print_stats_line(0.0, total_elapsed, self.total_bytes);
        eprintln!("- - - - - - - - - - - - - - - - - - - - - - - - -");
    }

    fn print_stats(&self, elapsed: f64, bytes: usize) {
        let now = Instant::now();
        let total_elapsed = now.duration_since(self.start_total).as_secs_f64();
        // iperf-like output uses start-end time for interval
        // approximate interval start as total - elapsed
        let interval_start = total_elapsed - elapsed;
        self.print_stats_line(interval_start, total_elapsed, bytes);
    }

    fn print_stats_line(&self, start: f64, end: f64, bytes: usize) {
        let transfer_mbytes = (bytes as f64) / (1024.0 * 1024.0);
        // 8 bits per byte
        let bitrate_mbps = (bytes as f64 * 8.0) / ((end - start) * 1_000_000.0);

        // [ ID] Interval           Transfer     Bitrate         Retr  Cwnd
        // [  5]   0.00-1.00   sec  11.2 MBytes  94.1 Mbits/sec    0   0.00 KBytes
        eprintln!(
            "[  5]   {:.2}-{:.2}   sec  {:.2} MBytes  {:.2} Mbits/sec    0   0.00 KBytes",
            start, end, transfer_mbytes, bitrate_mbps
        );
    }

    pub fn print_header(&self) {
        eprintln!("- - - - - - - - - - - - - - - - - - - - - - - - -");
        eprintln!("[ ID] Interval           Transfer     Bitrate         Retr  Cwnd");
    }
}
