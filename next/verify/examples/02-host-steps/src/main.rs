// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use verify_host_lib::{
    features::Features,
    orchestrator::{self, RunArgs, TestContext},
};

#[derive(Parser)]
#[command(name = "custom_host_runner")]
struct Cli {
    #[command(flatten)]
    pub args: RunArgs,
}

#[verify_macros::step_module]
pub mod custom_steps {
    use verify_macros::step;

    use super::*;

    #[step(r#"Host says hello to "([^"]+)""#)]
    async fn host_says_hello(w: &mut TestContext, name: String) -> Result<(), String> {
        println!("Hello, {} from host!", name);
        Ok(())
    }

    #[step(r#"Host checks for devices"#)]
    async fn host_checks_for_devices(_w: &mut TestContext) -> Result<(), String> {
        let output =
            std::process::Command::new("adb").arg("devices").output().map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err("Failed to run adb devices".to_string());
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        println!("Adb devices output:\n{}", stdout);
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let cli = Cli::parse();

    let mut features = Features::<TestContext>::new();
    verify_host_lib::adb_steps::register_steps(&mut features);
    verify_host_lib::android_steps::register_steps(&mut features);
    verify_host_lib::host_steps::register_steps(&mut features);
    verify_host_lib::netsim_steps::register_steps(&mut features);
    custom_steps::register_steps(&mut features);

    orchestrator::run(cli.args, features).await?;

    Ok(())
}
