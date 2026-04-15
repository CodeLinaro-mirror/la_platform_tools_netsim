use anyhow::{anyhow, Result};

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
            .ok_or_else(|| anyhow!("Actor not found or is not an Android Agent"))?;
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

/// STEP: Given ^@netsim is running$
async fn netsim_running(w: &mut TestContext) {
    w.log_step("@netsim", "GIVEN", "Is running");
}

/// STEP: When ^@netsim moves @(\S+) to ([\d\.]+), ([\d\.]+), ([\d\.]+)$
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

// Include generated glue code
include!(env!("NETSIM_STEPS_GLUE"));
