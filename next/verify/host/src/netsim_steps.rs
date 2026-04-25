// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Context, Result};
use features::{self, DataTable, World};
use netsim_proto::{access_point, frontend};
use protobuf::{well_known_types::empty::Empty, MessageField};
use verify_macros::{step, step_module};

use crate::{
    orchestrator::TestContext,
    types::{ClientParams, Throughput},
};

pub struct NetsimWorld;

impl NetsimWorld {
    pub fn new() -> Self {
        Self
    }

    pub async fn run_client(&mut self, _params: ClientParams) -> Result<Option<Throughput>> {
        Ok(None)
    }

    pub fn get_label(&self) -> String {
        "netsim".to_string()
    }

    pub fn set_silent(&self, _s: bool) {}
}

#[step_module]
pub mod steps {
    use super::*;

    const DEFAULT_POSITION_DELTA: f32 = 0.001;

    fn evaluate_condition(key: &str, actual: &str, expected: &str) -> Result<()> {
        if expected == "*" {
            return Ok(());
        }
        if expected.starts_with(">=") {
            let exp_val = expected[2..].parse::<f64>().context("Parse expected value")?;
            let act_val = actual.parse::<f64>().context("Parse actual value")?;
            anyhow::ensure!(
                act_val >= exp_val,
                "Expected '{}' >= {}, but got {}",
                key,
                exp_val,
                act_val
            );
        } else if expected.starts_with(">") {
            let exp_val = expected[1..].parse::<f64>().context("Parse expected value")?;
            let act_val = actual.parse::<f64>().context("Parse actual value")?;
            anyhow::ensure!(
                act_val > exp_val,
                "Expected '{}' > {}, but got {}",
                key,
                exp_val,
                act_val
            );
        } else if expected.starts_with("<=") {
            let exp_val = expected[2..].parse::<f64>().context("Parse expected value")?;
            let act_val = actual.parse::<f64>().context("Parse actual value")?;
            anyhow::ensure!(
                act_val <= exp_val,
                "Expected '{}' <= {}, but got {}",
                key,
                exp_val,
                act_val
            );
        } else if expected.starts_with("<") {
            let exp_val = expected[1..].parse::<f64>().context("Parse expected value")?;
            let act_val = actual.parse::<f64>().context("Parse actual value")?;
            anyhow::ensure!(
                act_val < exp_val,
                "Expected '{}' < {}, but got {}",
                key,
                exp_val,
                act_val
            );
        } else {
            anyhow::ensure!(
                actual.trim() == expected.trim(),
                "Expected '{}' to be '{}', but got '{}'",
                key,
                expected,
                actual
            );
        }
        Ok(())
    }

    #[step("@netsim is running")]
    async fn netsim_running(w: &mut TestContext) -> Result<()> {
        w.log_step("@netsim", "GIVEN", "Is running");
        Ok(())
    }

    #[step(r#"@netsim moves @(\S+) to ([\d\.]+), ([\d\.]+), ([\d\.]+)"#)]
    async fn netsim_move(w: &mut TestContext, actor: String, x: f32, y: f32, z: f32) -> Result<()> {
        let resolved_actor = w.resolve_placeholders(&format!("@{}", actor));
        let netsim_device =
            w.map_actor_to_netsim(&resolved_actor).context("got netsim device name")?;

        w.log_step(
            "@netsim",
            "->",
            &format!("Moves {} ({}) to {}, {}, {}", resolved_actor, netsim_device, x, y, z),
        );
        if w.is_dry_run {
            return Ok(());
        }

        let client = w.get_or_create_grpc_client().context("got grpc client")?;
        let resp = client.list_device(&Empty::new()).context("listed devices")?;
        let device = resp
            .devices
            .iter()
            .find(|d| d.name == netsim_device)
            .ok_or_else(|| anyhow!("Device '{}' not found in netsim devices", netsim_device))?;

        let mut req = frontend::PatchDeviceRequest::new();
        req.id = Some(device.id);

        let mut fields = frontend::patch_device_request::PatchDeviceFields::new();
        let mut pos = netsim_proto::model::Position::new();
        pos.x = x;
        pos.y = y;
        pos.z = z;
        fields.position = MessageField::some(pos);
        req.device = MessageField::some(fields);

        client.patch_device(&req).context("patched device")?;
        Ok(())
    }

    #[step(r#"Netsim creates Wi-Fi Access Point "([^"]+)" with protocol "([^"]+)""#)]
    async fn host_creates_ap(w: &mut TestContext, ssid: String, protocol: String) -> Result<()> {
        w.log_step("@netsim", "->", &format!("Creates AP '{}' with protocol '{}'", ssid, protocol));

        if w.is_dry_run {
            return Ok(());
        }
        let client = w.get_or_create_ap_client().context("got ap client")?;
        let mut ap = access_point::AccessPoint::new();
        ap.ssid = ssid;
        ap.hw_mode = protocol;
        let mut req = access_point::CreateAccessPointRequest::new();
        req.access_point = MessageField::some(ap);
        client.create(&req).context("created AP")?;
        Ok(())
    }

    #[step(
        r#"Netsim creates Wi-Fi Access Point "([^"]+)" with protocol "([^"]+)" and password "([^"]+)""#
    )]
    async fn host_creates_secured_ap(
        w: &mut TestContext,
        ssid: String,
        protocol: String,
        password: String,
    ) -> Result<()> {
        w.log_step(
            "@netsim",
            "->",
            &format!("Creates Secured AP '{}' with protocol '{}'", ssid, protocol),
        );

        if w.is_dry_run {
            return Ok(());
        }
        let client = w.get_or_create_ap_client().context("got ap client")?;
        let mut ap = access_point::AccessPoint::new();
        ap.ssid = ssid;
        ap.hw_mode = protocol;
        ap.wpa_passphrase = password;
        let mut req = access_point::CreateAccessPointRequest::new();
        req.access_point = MessageField::some(ap);
        client.create(&req).context("created secured AP")?;
        Ok(())
    }

    #[step(r#"Wi-Fi Access Point "([^"]+)" in netsim has protocol "([^"]+)""#)]
    async fn verify_ap_protocol(w: &mut TestContext, ssid: String, protocol: String) -> Result<()> {
        w.log_step("@netsim", "THEN", &format!("AP '{}' has protocol '{}'", ssid, protocol));

        if w.is_dry_run {
            return Ok(());
        }

        let client = w.get_or_create_ap_client().context("got ap client")?;
        let req = access_point::ListAccessPointsRequest::new();
        let resp = client.list(&req).context("listed APs")?;

        let found = resp.access_points.iter().any(|ap| ap.ssid == ssid && ap.hw_mode == protocol);

        if !found {
            anyhow::bail!("AP with SSID '{}' and protocol '{}' not found", ssid, protocol);
        }
        Ok(())
    }

    #[step(r#"Netsim removes Wi-Fi Access Point "([^"]+)""#)]
    async fn host_removes_ap(w: &mut TestContext, ssid: String) -> Result<()> {
        w.log_step("@netsim", "->", &format!("Removes AP '{}'", ssid));

        if w.is_dry_run {
            return Ok(());
        }
        let client = w.get_or_create_ap_client().context("got ap client")?;
        let req = access_point::ListAccessPointsRequest::new();
        let resp = client.list(&req).context("listed APs")?;

        let ap = resp
            .access_points
            .iter()
            .find(|ap| ap.ssid == ssid)
            .context(format!("AP with SSID '{}' not found", ssid))?;

        let mut del_req = access_point::DeleteAccessPointRequest::new();
        del_req.id = ap.id;
        client.delete(&del_req).context("deleted AP")?;
        Ok(())
    }

    #[step(r#"@netsim observes "([^"]+)" should be "([^"]+)""#)]
    async fn then_netsim_observes_is(
        w: &mut TestContext,
        key: String,
        expected_value: String,
    ) -> Result<()> {
        w.log_step(
            "@netsim",
            "THEN",
            &format!("observes \"{}\" should be \"{}\"", key, expected_value),
        );
        w.fetch_observables().await?;
        let actual_value = w
            .variables
            .get(&key)
            .ok_or_else(|| anyhow::anyhow!("Observable '{}' not found", key))?;
        evaluate_condition(&key, actual_value, &expected_value)?;
        Ok(())
    }

    #[step(r#"@netsim observes:?"#)]
    async fn then_netsim_observes_table(w: &mut TestContext, table: DataTable) -> Result<()> {
        w.log_step("@netsim", "THEN", "observes multiple fields");
        w.fetch_observables().await?;
        let expected_fields = features::vertical_table_to_map(&table);

        for (key, expected_value) in expected_fields {
            w.log_step("@netsim", "INFO", &format!("  | {} | {} |", key, expected_value));
            let actual_value = w
                .variables
                .get(&key)
                .ok_or_else(|| anyhow::anyhow!("Observable '{}' not found", key))?;
            evaluate_condition(&key, actual_value, &expected_value)?;
        }
        Ok(())
    }

    #[step(r#"Netsim creates BLE Beacon "([^"]+)" at ([\d\.]+), ([\d\.]+), ([\d\.]+)"#)]
    async fn host_creates_beacon(
        w: &mut TestContext,
        name: String,
        x: f32,
        y: f32,
        z: f32,
    ) -> Result<()> {
        host_creates_beacon_with_address(w, name, "".to_string(), x, y, z).await
    }

    #[step(r#"Netsim creates BLE Beacon "([^"]+)" with address "([^"]+)" at ([\d\.]+), ([\d\.]+), ([\d\.]+)"#)]
    async fn host_creates_beacon_with_address(
        w: &mut TestContext,
        name: String,
        address: String,
        x: f32,
        y: f32,
        z: f32,
    ) -> Result<()> {
        w.log_step(
            "@netsim",
            "->",
            &format!("Creates BLE Beacon '{}' at {}, {}, {}", name, x, y, z),
        );

        if w.is_dry_run {
            return Ok(());
        }

        let client = w.get_or_create_grpc_client().context("got grpc client")?;

        let mut pos = netsim_proto::model::Position::new();
        pos.x = x;
        pos.y = y;
        pos.z = z;

        let mut beacon_create = netsim_proto::model::chip_create::BleBeaconCreate::new();
        beacon_create.address = address.clone();

        let mut chip_create = netsim_proto::model::ChipCreate::new();
        chip_create.kind = protobuf::EnumOrUnknown::new(netsim_proto::common::ChipKind::BLUETOOTH);
        chip_create.name = name.clone();
        chip_create.address = address;
        chip_create.set_ble_beacon(beacon_create);

        let mut dev_create = netsim_proto::model::DeviceCreate::new();
        dev_create.name = name;
        dev_create.position = MessageField::some(pos);
        dev_create.chips.push(chip_create);

        let mut req = frontend::CreateDeviceRequest::new();
        req.device = MessageField::some(dev_create);

        client.create_device(&req).context("created beacon device")?;
        Ok(())
    }

    async fn verify_device_position_impl(
        w: &mut TestContext,
        actor: String,
        x: f32,
        y: f32,
        z: f32,
        delta: f32,
    ) -> Result<()> {
        let resolved_actor = w.resolve_placeholders(&format!("@{}", actor));
        let netsim_device =
            w.map_actor_to_netsim(&resolved_actor).context("got netsim device name")?;

        w.log_step(
            "@netsim",
            "THEN",
            &format!("Device '{}' ({}) is at {}, {}, {}", resolved_actor, netsim_device, x, y, z),
        );

        if w.is_dry_run {
            return Ok(());
        }

        let client = w.get_or_create_grpc_client().context("got grpc client")?;
        let resp = client.list_device(&Empty::new()).context("listed devices")?;
        let device = resp
            .devices
            .iter()
            .find(|d| d.name == netsim_device)
            .ok_or_else(|| anyhow!("Device '{}' not found in netsim devices", netsim_device))?;

        let pos = device.position.as_ref().context("device has position")?;

        if (pos.x - x).abs() > delta || (pos.y - y).abs() > delta || (pos.z - z).abs() > delta {
            anyhow::bail!(
                "Device '{}' position mismatch. Expected: {}, {}, {}. Actual: {}, {}, {}",
                netsim_device,
                x,
                y,
                z,
                pos.x,
                pos.y,
                pos.z
            );
        }
        Ok(())
    }

    #[step(r#"Device "([^"]+)" in netsim is at ([\d\.]+), ([\d\.]+), ([\d\.]+)"#)]
    async fn verify_device_position(
        w: &mut TestContext,
        actor: String,
        x: f32,
        y: f32,
        z: f32,
    ) -> Result<()> {
        verify_device_position_impl(w, actor, x, y, z, DEFAULT_POSITION_DELTA).await
    }

    #[step(
        r#"Device "([^"]+)" in netsim is at ([\d\.]+), ([\d\.]+), ([\d\.]+) with delta ([\d\.]+)"#
    )]
    async fn verify_device_position_with_delta(
        w: &mut TestContext,
        actor: String,
        x: f32,
        y: f32,
        z: f32,
        delta: f32,
    ) -> Result<()> {
        verify_device_position_impl(w, actor, x, y, z, delta).await
    }

    #[step(r#"Wi-Fi Access Point "([^"]+)" exists in netsim"#)]
    async fn verify_ap_exists(w: &mut TestContext, ssid: String) -> Result<()> {
        w.log_step("@netsim", "THEN", &format!("AP '{}' exists", ssid));

        if w.is_dry_run {
            return Ok(());
        }

        let client = w.get_or_create_ap_client().context("got ap client")?;
        let req = access_point::ListAccessPointsRequest::new();
        let resp = client.list(&req).context("listed APs")?;

        let found = resp.access_points.iter().any(|ap| ap.ssid == ssid);

        if !found {
            anyhow::bail!("AP with SSID '{}' not found", ssid);
        }
        Ok(())
    }

    #[step(r#"Netsim creates BLE Beacon "([^"]+)" with Tx Power "([^"]+)" at ([\d\.]+), ([\d\.]+), ([\d\.]+)"#)]
    async fn host_creates_beacon_with_tx_power(
        w: &mut TestContext,
        name: String,
        tx_power: String,
        x: f32,
        y: f32,
        z: f32,
    ) -> Result<()> {
        w.log_step(
            "@netsim",
            "->",
            &format!(
                "Creates BLE Beacon '{}' with Tx Power '{}' at {}, {}, {}",
                name, tx_power, x, y, z
            ),
        );

        if w.is_dry_run {
            return Ok(());
        }

        let client = w.get_or_create_grpc_client().context("got grpc client")?;

        let mut pos = netsim_proto::model::Position::new();
        pos.x = x;
        pos.y = y;
        pos.z = z;

        let power_level = match tx_power.to_lowercase().as_str() {
            "ultra_low" | "ultralow" => netsim_proto::model::chip::ble_beacon::advertise_settings::AdvertiseTxPower::ULTRA_LOW,
            "low" => netsim_proto::model::chip::ble_beacon::advertise_settings::AdvertiseTxPower::LOW,
            "medium" => netsim_proto::model::chip::ble_beacon::advertise_settings::AdvertiseTxPower::MEDIUM,
            "high" => netsim_proto::model::chip::ble_beacon::advertise_settings::AdvertiseTxPower::HIGH,
            _ => anyhow::bail!("Unknown Tx Power level: {}", tx_power),
        };

        let mut settings = netsim_proto::model::chip::ble_beacon::AdvertiseSettings::new();
        settings.set_tx_power_level(power_level);

        let mut beacon_create = netsim_proto::model::chip_create::BleBeaconCreate::new();
        beacon_create.settings = MessageField::some(settings);

        let mut chip_create = netsim_proto::model::ChipCreate::new();
        chip_create.kind = protobuf::EnumOrUnknown::new(netsim_proto::common::ChipKind::BLUETOOTH);
        chip_create.name = name.clone();
        chip_create.set_ble_beacon(beacon_create);

        let mut dev_create = netsim_proto::model::DeviceCreate::new();
        dev_create.name = name;
        dev_create.position = MessageField::some(pos);
        dev_create.chips.push(chip_create);

        let mut req = frontend::CreateDeviceRequest::new();
        req.device = MessageField::some(dev_create);

        client.create_device(&req).context("created beacon device")?;
        Ok(())
    }

    #[step(r#"Netsim version is "([^"]+)""#)]
    async fn verify_netsim_version(w: &mut TestContext, expected_version: String) -> Result<()> {
        w.log_step("@netsim", "THEN", &format!("Version is '{}'", expected_version));

        if w.is_dry_run {
            return Ok(());
        }

        let client = w.get_or_create_grpc_client().context("got grpc client")?;
        let resp = client
            .get_version(&protobuf::well_known_types::empty::Empty::new())
            .context("got version")?;

        if resp.version != expected_version {
            anyhow::bail!(
                "Version mismatch. Expected: {}. Actual: {}",
                expected_version,
                resp.version
            );
        }
        Ok(())
    }

    #[step(r#"Netsim has at least (\d+) devices"#)]
    async fn verify_netsim_device_count(w: &mut TestContext, expected_count: usize) -> Result<()> {
        w.log_step("@netsim", "THEN", &format!("Has at least {} devices", expected_count));

        if w.is_dry_run {
            return Ok(());
        }

        let client = w.get_or_create_grpc_client().context("got grpc client")?;
        let resp = client
            .list_device(&protobuf::well_known_types::empty::Empty::new())
            .context("listed devices")?;

        if resp.devices.len() < expected_count {
            anyhow::bail!(
                "Device count too low. Expected at least: {}. Actual: {}",
                expected_count,
                resp.devices.len()
            );
        }
        Ok(())
    }

    #[step(r#"Wi-Fi Access Point "([^"]+)" does not exist in netsim"#)]
    async fn verify_ap_does_not_exist(w: &mut TestContext, ssid: String) -> Result<()> {
        w.log_step("@netsim", "THEN", &format!("AP '{}' does not exist", ssid));

        if w.is_dry_run {
            return Ok(());
        }

        let client = w.get_or_create_ap_client().context("got ap client")?;
        let req = access_point::ListAccessPointsRequest::new();
        let resp = client.list(&req).context("listed APs")?;

        let found = resp.access_points.iter().any(|ap| ap.ssid == ssid);

        if found {
            anyhow::bail!("AP with SSID '{}' found, but expected not to exist", ssid);
        }
        Ok(())
    }

    // TODO: Add step `Device "{name}" in netsim has Tx Power "{tx_power}"`
    // once the Netsim backend populates `ble_beacon` field in `ListDevice`
    // response.
}

pub use steps::register_steps;
