// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{collections::HashSet, process::Stdio};

use anyhow::{Context, Result};
use tracing::{info, warn};

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
                anyhow::bail!(
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
                Self::identify_avd(&adb, &mut exec);

                if launch_netsim {
                    // HARDCODED: Enable pcap for debugging as requested
                    let args = self.netsim_args.clone().unwrap_or_default() + " --pcap";
                    if let Err(e) = exec.launch_netsim(&self.netsim_path, &Some(args)) {
                        println!("WARN   Failed to launch netsim: {}", e);
                    }
                    launch_netsim = false;
                }

                if is_verbose {
                    println!("INFO   @{} Installing ntest-agent APK...", exec.get_label());
                }

                // Best effort uninstall to clear state
                let _ = exec.uninstall_apk("com.android.netsim.agent");

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
                    .arg("com.android.netsim.agent.v2")
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

                if let Err(e) = exec.setup_feedback() {
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
                    println!(
                        "INFO   @{} Launching NTest instrumentation agent...",
                        exec.get_label()
                    );
                }
                if let Err(e) = exec.launch_agent() {
                    println!("WARN   Failed to launch agent on {}: {}", serial, e);
                    continue;
                }

                if let Err(e) = exec.wait_for_feedback() {
                    println!("WARN   Failed to connect to agent on {}: {}", serial, e);
                    continue;
                }

                new_agents.push(exec);
                known_serials.insert(serial);
            }

            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }

    fn identify_avd(adb: &str, exec: &mut AndroidDevice) {
        let serial = exec.serial.as_ref().unwrap();
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
    }
}

/// STEP: Given ^@(\S+) has (\d+) attached device(?:s)?$
async fn given_devices(w: &mut TestContext, actor: String, count: usize) {
    let actor = format!("@{}", actor);
    w.log_step(&actor, "GIVEN", &format!("Has {} or more attached devices", count));
    if w.is_dry_run {
        let known = w.get_all_serials();
        if known.len() >= count {
            return;
        }
        for i in 0..(count - known.len()) {
            let idx = known.len() + i;
            let serial = format!("emulator-{}", 5554 + (idx * 2));
            let agent = AndroidDevice::new(Some(serial), "mock_adb".to_string(), None).unwrap();
            w.register_android_actor(agent);
        }
        return;
    }
    let known = w.get_all_serials();
    if known.len() >= count {
        return;
    }

    // We get adb agent as immutable ref here
    let adb_agent = &w.adb;

    let new_agents =
        adb_agent.discover(known, count, w.is_verbose).await.expect("Discovery failed");

    for agent in new_agents {
        w.register_android_actor(agent);
    }
}

/// STEP: When ^(?:@adb)(?::(\S+))? connects to (?:open )?wifi "([^"]+)"$
async fn adb_connects_to_open_wifi(w: &mut TestContext, label: String, ssid: String) {
    let actor = if label.is_empty() { "@avd:1".to_string() } else { format!("@avd:{}", label) };
    w.log_step(&actor, "->", &format!("Connects to open WiFi network '{}'", ssid));

    if w.is_dry_run {
        return;
    }

    let android = w
        .get_android_actor(&actor)
        .unwrap_or_else(|| panic!("Actor {} not found or is not an Android Agent", actor));

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
        .expect("tokio spawn failed")
        .expect("Failed to execute adb shell cmd wifi");

    if !status.success() {
        panic!("Failed to connect to Wi-Fi network {}", ssid);
    }

    // Wait for Wi-Fi connection to show the SSID
    let mut connected = false;
    for i in 0..30 {
        let mut cmd = android.adb_command();
        cmd.arg("shell").arg("cmd").arg("wifi").arg("status");

        let out = tokio::task::spawn_blocking(move || cmd.output())
            .await
            .expect("tokio spawn failed")
            .expect("Failed to execute adb shell cmd wifi status");

        let stdout = String::from_utf8_lossy(&out.stdout);
        if stdout.contains(&ssid) {
            tracing::info!("Wi-Fi connected to {} after {} seconds", ssid, i);
            connected = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    if !connected {
        tracing::warn!("Timed out waiting for Wi-Fi to connect to {}!", ssid);
    }
}

/// STEP: When ^(?:@adb)(?::(\S+))? connects to wifi "([^"]+)" with password
/// "([^"]+)"$
async fn adb_connects_to_wifi(w: &mut TestContext, label: String, ssid: String, password: String) {
    let actor = if label.is_empty() { "@avd:1".to_string() } else { format!("@avd:{}", label) };
    w.log_step(&actor, "->", &format!("Connects to WiFi network '{}'", ssid));

    if w.is_dry_run {
        return;
    }

    let android = w
        .get_android_actor(&actor)
        .unwrap_or_else(|| panic!("Actor {} not found or is not an Android Agent", actor));

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
        .expect("tokio spawn failed")
        .expect("Failed to execute adb shell cmd wifi");

    if !status.success() {
        panic!("Failed to connect to Wi-Fi network {}", ssid);
    }
    // Wait for Wi-Fi connection to show the SSID
    let mut connected = false;
    for i in 0..30 {
        let mut cmd = android.adb_command();
        cmd.arg("shell").arg("cmd").arg("wifi").arg("status");

        let out = tokio::task::spawn_blocking(move || cmd.output())
            .await
            .expect("tokio spawn failed")
            .expect("Failed to execute adb shell cmd wifi status");

        let stdout = String::from_utf8_lossy(&out.stdout);
        if stdout.contains(&ssid) {
            info!("Wi-Fi connected to {} after {} seconds", ssid, i);
            connected = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    if !connected {
        warn!("Timed out waiting for Wi-Fi to connect to {}!", ssid);
    }
}

/// STEP: Then ^(?:@adb)(?::(\S+))? verifies connected wifi is "([^"]+)"$
async fn adb_verifies_wifi_ssid(w: &mut TestContext, label: String, expected_ssid: String) {
    let actor = if label.is_empty() { "@avd:1".to_string() } else { format!("@avd:{}", label) };
    w.log_step(&actor, "<-", &format!("Verifies connected WiFi is '{}'", expected_ssid));

    if w.is_dry_run {
        return;
    }

    let android = w
        .get_android_actor(&actor)
        .unwrap_or_else(|| panic!("Actor {} not found or is not an Android Agent", actor));

    let mut cmd = android.adb_command();
    cmd.arg("shell").arg("cmd").arg("wifi").arg("status");

    let out = tokio::task::spawn_blocking(move || cmd.output())
        .await
        .expect("tokio spawn failed")
        .expect("Failed to execute adb shell cmd wifi status");

    if !out.status.success() {
        panic!("Failed to get wifi status on {}", actor);
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    tracing::info!("wifi status output: {}", stdout);

    if !stdout.contains(&expected_ssid) {
        panic!("Expected SSID '{}' not found in wifi status: {}", expected_ssid, stdout);
    }
}

/// STEP: When ^(?:@adb)(?::(\S+))? disconnects from wifi$
async fn adb_disconnects_wifi(w: &mut TestContext, label: String) {
    let actor = if label.is_empty() { "@avd:1".to_string() } else { format!("@avd:{}", label) };
    w.log_step(&actor, "->", "Disconnects from WiFi");

    if w.is_dry_run {
        return;
    }

    let android = w
        .get_android_actor(&actor)
        .unwrap_or_else(|| panic!("Actor {} not found or is not an Android Agent", actor));

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
        .expect("tokio spawn failed")
        .expect("Failed to execute svc wifi disable");

    if !status.success() {
        panic!("Failed to disable Wi-Fi on {}", actor);
    }

    // Wait for Wi-Fi to actually turn off
    let mut off = false;
    for i in 0..30 {
        let mut cmd = android.adb_command();
        cmd.arg("shell").arg("cmd").arg("wifi").arg("status");

        let out = tokio::task::spawn_blocking(move || cmd.output())
            .await
            .expect("tokio spawn failed")
            .expect("Failed to execute adb shell cmd wifi status");

        let stdout = String::from_utf8_lossy(&out.stdout);
        if stdout.contains("disabled") || stdout.contains("inactive") {
            tracing::info!("Wi-Fi disabled after {} seconds", i);
            off = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    if !off {
        tracing::warn!("Timed out waiting for Wi-Fi to turn off!");
    }

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
        .expect("tokio spawn failed")
        .expect("Failed to execute svc wifi enable");

    if !status.success() {
        panic!("Failed to enable Wi-Fi on {}", actor);
    }

    // Wait for Wi-Fi to actually turn on and be ready
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
}

/// STEP: When ^(?:@adb)(?::(\S+))? forgets network "([^"]+)"$
async fn adb_forgets_network(w: &mut TestContext, label: String, ssid: String) {
    let actor = if label.is_empty() { "@avd:1".to_string() } else { format!("@avd:{}", label) };
    w.log_step(&actor, "->", &format!("Forgets network '{}'", ssid));

    if w.is_dry_run {
        return;
    }

    let android = w
        .get_android_actor(&actor)
        .unwrap_or_else(|| panic!("Actor {} not found or is not an Android Agent", actor));

    // 1. Get network list to find ID
    let mut list_cmd = android.adb_command();
    list_cmd.arg("shell").arg("cmd").arg("wifi").arg("list-networks");
    let output = list_cmd.output().expect("failed to execute cmd wifi list-networks");
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
        panic!("Network with SSID '{}' not found in saved networks", ssid);
    };

    // 2. Forget network
    let mut forget_cmd = android.adb_command();
    forget_cmd
        .arg("shell")
        .arg("cmd")
        .arg("wifi")
        .arg("forget-network")
        .arg(network_id.to_string());

    let output = forget_cmd.output().expect("failed to execute cmd wifi forget-network");
    if !output.status.success() {
        panic!(
            "Failed to forget network on {}: {}",
            actor,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// STEP: When ^(?:@adb)(?::(\S+))? disables cellular data$
pub async fn adb_disables_cellular(w: &mut TestContext, label: String) {
    let actor = if label.is_empty() { "@avd:1".to_string() } else { format!("@avd:{}", label) };

    let android = w
        .get_android_actor(&actor)
        .unwrap_or_else(|| panic!("Actor {} not found or is not an Android Agent", actor));

    info!("{} Disabling Cellular Data...", actor);

    if w.is_dry_run {
        return;
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
        .expect("tokio spawn failed")
        .expect("Failed to execute svc data disable");

    if !status.success() {
        panic!("Failed to disable cellular data on {}", actor);
    }
}

// Include generated glue code
include!(env!("ADB_STEPS_GLUE"));
