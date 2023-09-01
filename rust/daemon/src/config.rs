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

/// Configuration for netsim
use lazy_static::lazy_static;
use std::sync::{Once, RwLock};

static SET_DEV_CALLED: Once = Once::new();
static SET_PCAP_CALLED: Once = Once::new();
static SET_DISABLE_ADDRESS_REUSE_CALLED: Once = Once::new();

lazy_static! {
    static ref CONFIG: RwLock<Config> = RwLock::new(Config::new());
}

struct Config {
    pub dev: Option<bool>,
    pub pcap: Option<bool>,
    pub disable_address_reuse: Option<bool>,
}

impl Config {
    pub fn new() -> Self {
        Self { dev: None, pcap: None, disable_address_reuse: None }
    }
}

/// Get the flag of dev
pub fn get_dev() -> bool {
    let config = CONFIG.read().unwrap();
    config.dev.unwrap_or(false)
}

/// Set the flag of dev
pub fn set_dev(flag: bool) {
    SET_DEV_CALLED.call_once(|| {
        let mut config = CONFIG.write().unwrap();
        config.dev = Some(flag);
    });
}

/// Get the flag of pcap
pub fn get_pcap() -> bool {
    let config = CONFIG.read().unwrap();
    config.pcap.unwrap_or(false)
}

/// Set the flag of pcap
pub fn set_pcap(flag: bool) {
    SET_PCAP_CALLED.call_once(|| {
        let mut config = CONFIG.write().unwrap();
        config.pcap = Some(flag);
    });
}

/// Get the flag of disable_address_reuse
pub fn get_disable_address_reuse() -> bool {
    let config = CONFIG.read().unwrap();
    config.disable_address_reuse.unwrap_or(false)
}

/// Set the flag of disable_address_reuse
pub fn set_disable_address_reuse(flag: bool) {
    SET_DISABLE_ADDRESS_REUSE_CALLED.call_once(|| {
        let mut config = CONFIG.write().unwrap();
        config.disable_address_reuse = Some(flag);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dev() {
        // Check if default dev boolean is false
        assert!(!get_dev());

        // Check if set_dev changes the flag to true
        set_dev(true);
        assert!(get_dev());

        // Check if set_dev can only be called once
        set_dev(false);
        assert!(get_dev());
    }

    #[test]
    fn test_pcap() {
        // Check if default pcap boolean is false
        assert!(!get_pcap());

        // Check if set_pcap changes the flag to true
        set_pcap(true);
        assert!(get_pcap());

        // Check if set_pcap can only be called once
        set_pcap(false);
        assert!(get_pcap());
    }

    #[test]
    fn test_disable_address_reuse() {
        // Check if default disable_address_reuse boolean is false
        assert!(!get_disable_address_reuse());

        // Check if set_disable_address_reuse changes the flag to true
        set_disable_address_reuse(true);
        assert!(get_disable_address_reuse());

        // Check if set_disable_address_reuse can only be called once
        set_disable_address_reuse(false);
        assert!(get_disable_address_reuse());
    }
}
