// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Context, Result};
use features::{self, DataTable};
use netsim_proto::{
    access_point, access_point_grpc::AccessPointServiceClient, frontend,
    frontend_grpc::FrontendServiceClient,
};
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

    /// Maps an orchestrator actor label (e.g. @android:2) to a netsim device
    /// label (e.g. Pixel 6 2) by querying the AVD name from the guest via
    /// ADB.
    pub fn map_actor_to_netsim(&self, w: &TestContext, actor: &str) -> Result<String> {
        let android = w
            .android
            .get(actor)
            .ok_or_else(|| anyhow!("Actor not found or is not an Android VBS"))?;
        let adb_path = &android.adb_path;
        let serial = android
            .serial
            .as_ref()
            .ok_or_else(|| anyhow!("Android device must have a serial for netsim mapping"))?;

        let mut cmd = std::process::Command::new(adb_path);
        cmd.arg("-s").arg(serial).arg("shell").arg("getprop").arg("ro.boot.qemu.avd_name");

        let output = cmd.output()?;
        if output.status.success() {
            let avd_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !avd_name.is_empty() {
                // Netsim replaces underscores with spaces in AVD names
                return Ok(avd_name.replace('_', " "));
            } else {
                Err(anyhow!("Could not find adb device name"))
            }
        } else {
            Err(anyhow!("Failed to call adb shell: {}", String::from_utf8_lossy(&output.stdout)))
        }
    }
}

#[step_module]
pub mod steps {
    use super::*;

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
            w.netsim.map_actor_to_netsim(w, &resolved_actor).context("got netsim device name")?;

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

    async fn fetch_netsim_observables_internal(w: &mut TestContext) -> Result<()> {
        if w.is_dry_run {
            w.variables.insert("connected-devices".to_string(), "1".to_string());
            return Ok(());
        }
        let (actual_count, version) = {
            let client = w.get_or_create_grpc_client().context("got grpc client")?;
            let resp = client.list_device(&Empty::new()).context("listed devices")?;
            let version_resp = client.get_version(&Empty::new()).context("got version")?;
            (resp.devices.len(), version_resp.version)
        };
        w.variables.insert("connected-devices".to_string(), actual_count.to_string());
        w.variables.insert("netsim-version".to_string(), version);
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
        fetch_netsim_observables_internal(w).await?;
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
        fetch_netsim_observables_internal(w).await?;
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
}

pub use steps::register_steps;
