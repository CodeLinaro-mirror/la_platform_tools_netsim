use std::{collections::HashSet, process::Stdio};

use anyhow::{Context, Result};

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

                println!("INFO   @{} Installing ntest-agent APK...", exec.get_label());
                // Best effort uninstall to clear state
                let _ = exec.uninstall_apk("com.android.netsim.agent");

                if let Err(e) = exec.install_apk() {
                    println!("WARN   Failed to install APK on {}: {}", serial, e);
                    // Retry in next loop iteration
                    continue;
                }

                if let Err(e) = exec.setup_feedback() {
                    println!("WARN   Failed to setup feedback on {}: {}", serial, e);
                    continue;
                }

                println!("INFO   Waiting 5s for package registration on {}...", exec.get_label());
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;

                println!("INFO   @{} Launching NTest instrumentation agent...", exec.get_label());
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
    }
}

/// STEP: Given @(\S+) has (\d+) attached device(?:s)?
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

    let new_agents = adb_agent.discover(known, count).await.expect("Discovery failed");

    for agent in new_agents {
        w.register_android_actor(agent);
    }
}

// Include generated glue code
include!(env!("ADB_STEPS_GLUE"));
