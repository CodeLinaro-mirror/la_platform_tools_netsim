use crate::error::WifiError;
use netsim_packets::ieee80211::MacAddress;

pub type WifiResult<T> = Result<T, WifiError>;

#[derive(Clone, Debug)]
pub struct Station {
    pub client_id: u32,
    pub addr: MacAddress,
    pub hwsim_addr: MacAddress,
    pub freq: u32,
}

impl Station {
    pub fn new(client_id: u32, addr: MacAddress, hwsim_addr: MacAddress) -> Self {
        Self { client_id, addr, hwsim_addr, freq: 0 }
    }

    pub fn update_freq(&mut self, freq: u32) {
        self.freq = freq;
    }
}

#[derive(Clone, Debug)]
pub struct Client {
    pub enabled: bool,
    pub tx_count: u32,
    pub rx_count: u32,
}

impl Client {
    pub fn new() -> Self {
        Self { enabled: true, tx_count: 0, rx_count: 0 }
    }
}
