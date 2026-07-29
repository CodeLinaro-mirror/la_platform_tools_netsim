// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{fs::File, io::BufReader};

use serde::Deserialize;
use tracing::{info, warn};

#[derive(Deserialize, Debug)]
struct InstanceConfig {
    ril_ipaddr: String,
    ril_prefixlen: u32,
    ril_gateway: String,
    ril_dns: String,
}

#[derive(Deserialize, Debug)]
struct CuttlefishConfig {
    instances: std::collections::HashMap<String, InstanceConfig>,
}

pub struct CuttlefishRilConfig {
    pub ip_address: String,
    pub prefixlen: u32,
    pub gateway: String,
    pub dns: String,
}

pub fn read_cuttlefish_config() -> Option<CuttlefishRilConfig> {
    let config_path = std::env::var("CUTTLEFISH_CONFIG_FILE").ok()?;
    let instance_id = std::env::var("CUTTLEFISH_INSTANCE").unwrap_or_else(|_| "1".to_string());
    read_cuttlefish_config_with_params(&config_path, &instance_id)
}

pub fn read_cuttlefish_config_with_params(
    config_path: &str,
    instance_id: &str,
) -> Option<CuttlefishRilConfig> {
    info!("Reading Cuttlefish config from {} for instance {}", config_path, instance_id);

    let file = match File::open(config_path) {
        Ok(f) => f,
        Err(e) => {
            warn!("Failed to open cuttlefish config {}: {}", config_path, e);
            return None;
        }
    };

    let reader = BufReader::new(file);
    let config: CuttlefishConfig = match serde_json::from_reader(reader) {
        Ok(c) => c,
        Err(e) => {
            warn!("Failed to parse cuttlefish config {}: {}", config_path, e);
            return None;
        }
    };

    let instance_config = config.instances.get(instance_id)?;

    Some(CuttlefishRilConfig {
        ip_address: instance_config.ril_ipaddr.clone(),
        prefixlen: instance_config.ril_prefixlen,
        gateway: instance_config.ril_gateway.clone(),
        dns: instance_config.ril_dns.clone(),
    })
}
