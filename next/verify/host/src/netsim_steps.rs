// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use verify_macros::{step, step_module};

use crate::{
    orchestrator::TestContext,
    types::{ClientParams, Throughput},
};

pub struct NetsimWorld {
    pub netsim_path: Option<String>,
}

impl NetsimWorld {
    pub fn new(netsim_path: Option<String>) -> Self {
        Self { netsim_path }
    }

    pub async fn run_client(&mut self, _params: ClientParams) -> Result<Option<Throughput>> {
        Ok(None)
    }

    /// Global reset for the netsim simulation environment.
    pub async fn reset_actor(&mut self) -> Result<()> {
        let cli_path = self
            .netsim_path
            .as_ref()
            .ok_or_else(|| anyhow!("Cannot reset devices without netsim cli"))?;
        let mut cmd = std::process::Command::new(cli_path);
        cmd.arg("reset");
        if cmd.status()?.success() {
            Ok(())
        } else {
            let output = cmd.output()?;
            Err(anyhow!("Failed to reset device: {}", String::from_utf8_lossy(&output.stdout)))
        }
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

    pub fn move_device(
        &self,
        w: &TestContext,
        netsim_device: String,
        x: f32,
        y: f32,
        z: f32,
    ) -> Result<()> {
        let cli_path = w
            .netsim
            .netsim_path
            .as_ref()
            .ok_or_else(|| anyhow!("Cannot move devices without netsim cli"))?;

        let mut cmd = std::process::Command::new(cli_path);
        cmd.arg("move")
            .arg(&netsim_device)
            .arg(x.to_string())
            .arg(y.to_string())
            .arg(z.to_string());

        if cmd.status()?.success() {
            Ok(())
        } else {
            let output = cmd.output()?;
            Err(anyhow!("Failed to move device: {}", String::from_utf8_lossy(&output.stdout)))
        }
    }
}

#[step_module]
pub mod steps {
    use super::*;

    #[step("@netsim is running")]
    async fn netsim_running(w: &mut TestContext) {
        w.log_step("@netsim", "GIVEN", "Is running");
    }

    #[step(r#"@netsim moves @(\S+) to ([\d\.]+), ([\d\.]+), ([\d\.]+)"#)]
    async fn netsim_move(w: &mut TestContext, actor: String, x: f32, y: f32, z: f32) {
        let resolved_actor = w.resolve_placeholders(&format!("@{}", actor));
        let netsim_device =
            w.netsim.map_actor_to_netsim(w, &resolved_actor).expect("got netsim device name");

        w.log_step(
            "@netsim",
            "->",
            &format!("Moves {} ({}) to {}, {}, {}", resolved_actor, netsim_device, x, y, z),
        );

        if w.is_dry_run {
            return;
        }

        w.netsim.move_device(w, netsim_device, x, y, z).expect("moved successfully");
    }

    #[step(r#"Netsim creates Wi-Fi Access Point "([^"]+)" with protocol "([^"]+)""#)]
    async fn host_creates_ap(w: &mut TestContext, ssid: String, protocol: String) {
        w.log_step("@netsim", "->", &format!("Creates AP '{}' with protocol '{}'", ssid, protocol));

        if w.is_dry_run {
            return;
        }

        let cli_path = w.netsim.netsim_path.as_ref().expect("got netsim cli path");
        let mut cmd = std::process::Command::new(cli_path);
        cmd.arg("ap").arg("create").arg("--ssid").arg(&ssid).arg("--protocol").arg(&protocol);

        let output = cmd.output().expect("failed to execute netsim ap create");
        if !output.status.success() {
            panic!("Failed to create AP: {}", String::from_utf8_lossy(&output.stderr));
        }
    }

    #[step(
        r#"Netsim creates Wi-Fi Access Point "([^"]+)" with protocol "([^"]+)" and password "([^"]+)""#
    )]
    async fn host_creates_secured_ap(
        w: &mut TestContext,
        ssid: String,
        protocol: String,
        password: String,
    ) {
        w.log_step(
            "@netsim",
            "->",
            &format!("Creates Secured AP '{}' with protocol '{}'", ssid, protocol),
        );

        if w.is_dry_run {
            return;
        }

        let cli_path = w.netsim.netsim_path.as_ref().expect("got netsim cli path");
        let mut cmd = std::process::Command::new(cli_path);
        cmd.arg("ap")
            .arg("create")
            .arg("--ssid")
            .arg(&ssid)
            .arg("--protocol")
            .arg(&protocol)
            .arg("--password")
            .arg(&password);

        let output = cmd.output().expect("failed to execute netsim ap create");
        if !output.status.success() {
            panic!("Failed to create Secured AP: {}", String::from_utf8_lossy(&output.stderr));
        }
    }

    #[step(r#"Wi-Fi Access Point "([^"]+)" in netsim has protocol "([^"]+)""#)]
    async fn verify_ap_protocol(w: &mut TestContext, ssid: String, protocol: String) {
        w.log_step("@netsim", "THEN", &format!("AP '{}' has protocol '{}'", ssid, protocol));

        if w.is_dry_run {
            return;
        }

        let cli_path = w.netsim.netsim_path.as_ref().expect("got netsim cli path");
        let mut cmd = std::process::Command::new(cli_path);
        cmd.arg("ap").arg("list");

        let output = cmd.output().expect("failed to execute netsim ap list");
        let stdout = String::from_utf8_lossy(&output.stdout);

        let expected_phy_mode = format!("802.11{}", protocol);
        let mut found = false;

        for line in stdout.lines() {
            if line.contains(&ssid) {
                let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
                if parts.len() >= 7 {
                    let phy_mode = parts[6];
                    if phy_mode == expected_phy_mode {
                        found = true;
                        break;
                    }
                }
            }
        }

        if !found {
            panic!(
                "AP '{}' with protocol '{}' not found in netsim ap list output:\n{}",
                ssid, protocol, stdout
            );
        }
    }

    #[step(r#"Netsim removes Wi-Fi Access Point "([^"]+)""#)]
    async fn host_removes_ap(w: &mut TestContext, ssid: String) {
        w.log_step("@netsim", "->", &format!("Removes AP '{}'", ssid));

        if w.is_dry_run {
            return;
        }

        let cli_path = w.netsim.netsim_path.as_ref().expect("got netsim cli path");

        // 1. Get AP list to find ID
        let mut list_cmd = std::process::Command::new(cli_path);
        list_cmd.arg("ap").arg("list");
        let output = list_cmd.output().expect("failed to execute netsim ap list");
        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut id: Option<u32> = None;
        for line in stdout.lines() {
            if line.contains(&ssid) {
                let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
                if !parts.is_empty() {
                    if let Ok(parsed_id) = parts[0].parse::<u32>() {
                        id = Some(parsed_id);
                        break;
                    }
                }
            }
        }

        let id =
            id.unwrap_or_else(|| panic!("AP with SSID '{}' not found in netsim ap list", ssid));

        // 2. Remove AP
        let mut remove_cmd = std::process::Command::new(cli_path);
        remove_cmd.arg("ap").arg("remove").arg(id.to_string());

        let output = remove_cmd.output().expect("failed to execute netsim ap remove");
        if !output.status.success() {
            panic!("Failed to remove AP: {}", String::from_utf8_lossy(&output.stderr));
        }
    }
}

pub use steps::register_steps;
