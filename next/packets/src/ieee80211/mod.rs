pub mod frame;
pub mod ie;
pub mod json;
pub mod util;

pub use frame::*;
pub use ie::*;
pub use json::*;
pub use util::*;

mod tests;

/// Returns the raw bytes of the golden CCMP pcap file used for testing.
/// This allows other crates (like hostapd-rs) to use the same test vector without path resolution issues.
pub fn get_golden_ccmp_pcap() -> &'static [u8] {
    include_bytes!("test_data/golden_ccmp.pcap")
}
