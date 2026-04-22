// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashSet, process::Stdio};

use anyhow::{self, bail, Context, Result};
use tracing::info;
use verify_macros::{step, step_module};

use crate::{
    android_steps::AndroidDevice,
    orchestrator::TestContext,
    types::{ClientParams, Throughput},
};

pub struct AdbWorld {
    pub android_home: Option<String>,
    pub apk_path: Option<String>,
    pub netsim_path: Option<String>,
    pub netsim_args: Option<String>,
}

impl AdbWorld {
    pub async fn run_client(&mut self, _params: ClientParams) -> Result<Option<Throughput>> {
        Ok(None)
    }

    pub async fn reset_actor(&mut self) -> Result<()> {
        Ok(())
    }

    pub fn get_label(&self) -> String {
        "adb".to_string()
    }

    pub fn set_silent(&self, _s: bool) {}
}

impl AdbWorld {
    pub fn new(
        android_home: Option<String>,
        apk_path: Option<String>,
        netsim_path: Option<String>,
        netsim_args: Option<String>,
    ) -> Self {
        Self { android_home, apk_path, netsim_path, netsim_args }
    }

    pub async fn discover(
        &self,
        mut known_serials: HashSet<String>,
        expected: usize,
        is_verbose: bool,
    ) -> Result<Vec<AndroidDevice>> {
        let mut new_agents = Vec::new();
        let start = std::time::Instant::now();
        let adb = AndroidDevice::find_adb(self.android_home.as_deref());

        // Netsim launch logic: Only launch if we haven't found any devices yet (implied
        // fresh run) or if we decide so. Here we rely on
        // `known_serials.is_empty()` meaning "first discovery".
        let mut launch_netsim = known_serials.is_empty();

        loop {
            if known_serials.len() >= expected {
                return Ok(new_agents);
            }

            if start.elapsed() > std::time::Duration::from_secs(120) {
                bail!(
                    "Timeout waiting for devices. Expected {}, found {}. Waited 120s.",
                    expected,
                    known_serials.len()
                );
            }

            let out = std::process::Command::new(&adb)
                .arg("devices")
                .output()
                .context("Failed to execute adb devices")?;

            for line in String::from_utf8_lossy(&out.stdout).lines().skip(1) {
                if known_serials.len() >= expected {
                    break;
                }
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 2 || parts[1] != "device" {
                    continue;
                }
                let serial = parts[0].to_string();

                if known_serials.contains(&serial) {
                    continue;
                }

                // New device found
                let mut exec =
                    AndroidDevice::new(Some(serial.clone()), adb.clone(), self.apk_path.clone())?;
                Self::identify_avd(&adb, &mut exec)?;

                if launch_netsim {
                    // HARDCODED: Enable pcap for debugging as requested
                    let args = self.netsim_args.clone().unwrap_or_default() + " --pcap";
                    if let Err(e) = exec.launch_netsim(&self.netsim_path, &Some(args)) {
                        println!("WARN   Failed to launch netsim: {}", e);
                    }
                    launch_netsim = false;
                }

                if is_verbose {
                    println!("INFO   @{} Installing vbs APK...", exec.get_label());
                }

                // Best effort uninstall to clear state
                let _ = exec.uninstall_apk("com.android.verify.vbs");

                if let Err(e) = exec.install_apk() {
                    println!("WARN   Failed to install APK on {}: {}", serial, e);
                    // Retry in next loop iteration
                    continue;
                }

                // Grant special permission WRITE_SETTINGS
                let _ = exec
                    .adb_command()
                    .arg("shell")
                    .arg("appops")
                    .arg("set")
                    .arg("com.android.verify.vbs")
                    .arg("WRITE_SETTINGS")
                    .arg("allow")
                    .status();

                // Enable location services
                let _ = exec
                    .adb_command()
                    .arg("shell")
                    .arg("cmd")
                    .arg("location")
                    .arg("set-location-enabled")
                    .arg("true")
                    .status();

                if let Err(e) = exec.setup_feedback().await {
                    println!("WARN   Failed to setup feedback on {}: {}", serial, e);
                    continue;
                }

                if is_verbose {
                    println!(
                        "INFO   Waiting 5s for package registration on {}...",
                        exec.get_label()
                    );
                }
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;

                if is_verbose {
                    println!("INFO   @{} Launching VBS instrumentation...", exec.get_label());
                }
                if let Err(e) = exec.launch_agent() {
                    println!("WARN   Failed to launch VBS on {}: {}", serial, e);
                    continue;
                }

                if let Err(e) = exec.wait_for_feedback().await {
                    println!("WARN   Failed to connect to VBS on {}: {}", serial, e);
                    continue;
                }

                new_agents.push(exec);
                known_serials.insert(serial);
            }

            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }

    fn identify_avd(adb: &str, exec: &mut AndroidDevice) -> Result<()> {
        let serial = exec.serial.as_ref().context("Missing serial")?;
        if let Ok(out) = std::process::Command::new(adb)
            .arg("-s")
            .arg(serial)
            .arg("emu")
            .arg("avd")
            .arg("name")
            .stderr(Stdio::null())
            .output()
        {
            let name = String::from_utf8_lossy(&out.stdout)
                .lines()
                .last()
                .unwrap_or("")
                .trim()
                .to_string();
            if !name.is_empty() && name != "OK" {
                exec.avd_name = Some(name);
            }
        }

        // Log the SDK version for better diagnostics
        if let Ok(out) = std::process::Command::new(adb)
            .arg("-s")
            .arg(serial)
            .arg("shell")
            .arg("getprop")
            .arg("ro.build.version.sdk")
            .output()
        {
            let sdk = String::from_utf8_lossy(&out.stdout).trim().to_string();
            tracing::info!("Device {} has SDK version: {}", serial, sdk);
            eprintln!("Device {} has SDK version: {}", serial, sdk);
        }
        Ok(())
    }
}

#[step_module]
pub mod steps {
    use super::*;

    async fn poll_wifi_status(
        android: &AndroidDevice,
        check: impl Fn(&str) -> bool,
        success_msg: &str,
        timeout_msg: &str,
    ) -> Result<()> {
        for i in 0..30 {
            let mut cmd = android.adb_command();
            cmd.arg("shell").arg("cmd").arg("wifi").arg("status");

            let out = match tokio::task::spawn_blocking(move || cmd.output()).await {
                Ok(Ok(output)) => output,
                _ => {
                    tracing::warn!("Failed to get wifi status");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                }
            };

            let stdout = String::from_utf8_lossy(&out.stdout);
            if check(&stdout) {
                tracing::info!("{} after {} seconds", success_msg, i);
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        bail!("{}", timeout_msg)
    }

    #[step(r#"@(\S+) has (\d+) attached device(?:s)?"#)]
    async fn given_devices(w: &mut TestContext, actor: String, count: usize) -> Result<()> {
        let actor = format!("@{}", actor);
        w.log_step(&actor, "GIVEN", &format!("Has {} or more attached devices", count));
        if w.is_dry_run {
            let known = w.get_all_serials();
            if known.len() >= count {
                return Ok(());
            }
            for i in 0..(count - known.len()) {
                let idx = known.len() + i;
                let serial = format!("emulator-{}", 5554 + (idx * 2));
                let agent = AndroidDevice::new(Some(serial), "mock_adb".to_string(), None)?;
                w.register_android_actor(agent);
            }
            return Ok(());
        }
        let known = w.get_all_serials();
        if known.len() >= count {
            return Ok(());
        }

        // We get adb agent as immutable ref here
        let adb_agent = &w.adb;

        let new_agents =
            adb_agent.discover(known, count, w.is_verbose).await.context("Discovery failed")?;

        for agent in new_agents {
            w.register_android_actor(agent);
        }
        Ok(())
    }

    #[step(r#"(?:@adb)(?::(\S+))? connects to (?:open )?wifi "([^"]+)""#)]
    async fn adb_connects_to_open_wifi(
        w: &mut TestContext,
        label: String,
        ssid: String,
    ) -> Result<()> {
        let actor = if label.is_empty() { "@avd:1".to_string() } else { format!("@avd:{}", label) };
        w.log_step(&actor, "->", &format!("Connects to open WiFi network '{}'", ssid));

        if w.is_dry_run {
            return Ok(());
        }

        let android = w
            .get_android_actor(&actor)
            .context(format!("Actor {} not found or is not an Android VBS", actor))?;

        let mut connect_cmd = android.adb_command();
        connect_cmd
            .arg("shell")
            .arg("cmd")
            .arg("wifi")
            .arg("connect-network")
            .arg(&ssid)
            .arg("open")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());

        let status = tokio::task::spawn_blocking(move || connect_cmd.status())
            .await
            .context("tokio spawn failed")?
            .context("Failed to execute adb shell cmd wifi")?;

        if !status.success() {
            bail!("Failed to connect to Wi-Fi network {}", ssid);
        }

        // Wait for Wi-Fi connection to show the SSID
        poll_wifi_status(
            android,
            |stdout| stdout.contains(&ssid),
            &format!("Wi-Fi connected to {}", ssid),
            &format!("Timed out waiting for Wi-Fi to connect to {}!", ssid),
        )
        .await?;
        Ok(())
    }

    #[step(r#"(?:@adb)(?::(\S+))? connects to wifi "([^"]+)" with password "([^"]+)""#)]
    async fn adb_connects_to_wifi(
        w: &mut TestContext,
        label: String,
        ssid: String,
        password: String,
    ) -> Result<()> {
        let actor = if label.is_empty() { "@avd:1".to_string() } else { format!("@avd:{}", label) };
        w.log_step(&actor, "->", &format!("Connects to WiFi network '{}'", ssid));

        if w.is_dry_run {
            return Ok(());
        }

        let android = w
            .get_android_actor(&actor)
            .context(format!("Actor {} not found or is not an Android VBS", actor))?;

        let mut connect_cmd = android.adb_command();
        connect_cmd
            .arg("shell")
            .arg("cmd")
            .arg("wifi")
            .arg("connect-network")
            .arg(&ssid)
            .arg("wpa2")
            .arg(&password)
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let status = tokio::task::spawn_blocking(move || connect_cmd.status())
            .await
            .context("tokio spawn failed")?
            .context("Failed to execute adb shell cmd wifi")?;

        if !status.success() {
            bail!("Failed to connect to Wi-Fi network {}", ssid);
        }
        // Wait for Wi-Fi connection to show the SSID
        poll_wifi_status(
            android,
            |stdout| stdout.contains(&ssid),
            &format!("Wi-Fi connected to {}", ssid),
            &format!("Timed out waiting for Wi-Fi to connect to {}!", ssid),
        )
        .await?;
        Ok(())
    }

    #[step(r#"(?:@adb)(?::(\S+))? verifies connected wifi is "([^"]+)""#)]
    async fn adb_verifies_wifi_ssid(
        w: &mut TestContext,
        label: String,
        expected_ssid: String,
    ) -> Result<()> {
        let actor = if label.is_empty() { "@avd:1".to_string() } else { format!("@avd:{}", label) };
        w.log_step(&actor, "<-", &format!("Verifies connected WiFi is '{}'", expected_ssid));

        if w.is_dry_run {
            return Ok(());
        }

        let android = w
            .get_android_actor(&actor)
            .context(format!("Actor {} not found or is not an Android VBS", actor))?;

        let mut cmd = android.adb_command();
        cmd.arg("shell").arg("cmd").arg("wifi").arg("status");

        let out = tokio::task::spawn_blocking(move || cmd.output())
            .await
            .context("tokio spawn failed")?
            .context("Failed to execute adb shell cmd wifi status")?;

        if !out.status.success() {
            bail!("Failed to get wifi status on {}", actor);
        }

        let stdout = String::from_utf8_lossy(&out.stdout);
        tracing::info!("wifi status output: {}", stdout);

        if !stdout.contains(&expected_ssid) {
            bail!("Expected SSID '{}' not found in wifi status: {}", expected_ssid, stdout);
        }
        Ok(())
    }

    #[step(r#"(?:@adb)(?::(\S+))? disconnects from wifi"#)]
    async fn adb_disconnects_wifi(w: &mut TestContext, label: String) -> Result<()> {
        let actor = if label.is_empty() { "@avd:1".to_string() } else { format!("@avd:{}", label) };
        w.log_step(&actor, "->", "Disconnects from WiFi");

        if w.is_dry_run {
            return Ok(());
        }

        let android = w
            .get_android_actor(&actor)
            .context(format!("Actor {} not found or is not an Android VBS", actor))?;

        // Disable Wi-Fi
        let mut disable_cmd = android.adb_command();
        disable_cmd
            .arg("shell")
            .arg("cmd")
            .arg("wifi")
            .arg("set-wifi-enabled")
            .arg("disabled")
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let status = tokio::task::spawn_blocking(move || disable_cmd.status())
            .await
            .context("tokio spawn failed")?
            .context("Failed to execute svc wifi disable")?;

        if !status.success() {
            bail!("Failed to disable Wi-Fi on {}", actor);
        }

        // Wait for Wi-Fi to actually turn off
        poll_wifi_status(
            android,
            |stdout| stdout.contains("disabled") || stdout.contains("inactive"),
            "Wi-Fi disabled",
            "Timed out waiting for Wi-Fi to turn off!",
        )
        .await?;

        // Enable Wi-Fi
        let mut enable_cmd = android.adb_command();
        enable_cmd
            .arg("shell")
            .arg("svc")
            .arg("wifi")
            .arg("enable")
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let status = tokio::task::spawn_blocking(move || enable_cmd.status())
            .await
            .context("tokio spawn failed")?
            .context("Failed to execute svc wifi enable")?;

        if !status.success() {
            bail!("Failed to enable Wi-Fi on {}", actor);
        }

        // Wait for Wi-Fi to actually turn on and be ready
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        Ok(())
    }

    #[step(r#"(?:@adb)(?::(\S+))? forgets network "([^"]+)""#)]
    async fn adb_forgets_network(w: &mut TestContext, label: String, ssid: String) -> Result<()> {
        let actor = if label.is_empty() { "@avd:1".to_string() } else { format!("@avd:{}", label) };
        w.log_step(&actor, "->", &format!("Forgets network '{}'", ssid));

        if w.is_dry_run {
            return Ok(());
        }

        let android = w
            .get_android_actor(&actor)
            .context(format!("Actor {} not found or is not an Android VBS", actor))?;

        // 1. Get network list to find ID
        let mut list_cmd = android.adb_command();
        list_cmd.arg("shell").arg("cmd").arg("wifi").arg("list-networks");
        let output = list_cmd.output().context("failed to execute cmd wifi list-networks")?;
        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut network_id: Option<u32> = None;
        for line in stdout.lines() {
            if line.contains(&ssid) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if !parts.is_empty() {
                    if let Ok(parsed_id) = parts[0].parse::<u32>() {
                        network_id = Some(parsed_id);
                        break;
                    }
                }
            }
        }

        let Some(network_id) = network_id else {
            bail!("Network with SSID '{}' not found in saved networks", ssid);
        };

        // 2. Forget network
        let mut forget_cmd = android.adb_command();
        forget_cmd
            .arg("shell")
            .arg("cmd")
            .arg("wifi")
            .arg("forget-network")
            .arg(network_id.to_string());

        let output = forget_cmd.output().context("failed to execute cmd wifi forget-network")?;
        if !output.status.success() {
            bail!(
                "Failed to forget network on {}: {}",
                actor,
                String::from_utf8_lossy(&output.stderr)
            );
        }
        Ok(())
    }

    #[step(r#"(?:@adb)(?::(\S+))? disables cellular data"#)]
    async fn adb_disables_cellular(w: &mut TestContext, label: String) -> Result<()> {
        let actor = if label.is_empty() { "@avd:1".to_string() } else { format!("@avd:{}", label) };

        let android = w
            .get_android_actor(&actor)
            .context(format!("Actor {} not found or is not an Android VBS", actor))?;

        info!("{} Disabling Cellular Data...", actor);

        if w.is_dry_run {
            return Ok(());
        }

        let mut svc_cmd = android.adb_command();
        svc_cmd
            .arg("shell")
            .arg("svc")
            .arg("data")
            .arg("disable")
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let status = tokio::task::spawn_blocking(move || svc_cmd.status())
            .await
            .context("tokio spawn failed")?
            .context("Failed to execute svc data disable")?;

        if !status.success() {
            bail!("Failed to disable cellular data on {}", actor);
        }
        Ok(())
    }
}

pub use steps::register_steps;
