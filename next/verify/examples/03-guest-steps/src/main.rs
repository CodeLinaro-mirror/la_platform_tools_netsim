// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use clap::Parser;
use verify_host_lib::{
    features::Features,
    orchestrator::{self, RunArgs, TestContext},
};

#[derive(Parser)]
#[command(name = "custom_runner")]
struct Cli {
    #[command(flatten)]
    pub args: RunArgs,
}

// Local helper function to avoid borrow checker issues in macro
async fn execute_guest_step_helper(
    w: &mut TestContext,
    actor: String,
    name: String,
) -> Result<(), String> {
    let step = format!("Android says hello to \"{}\"", name);
    let is_dry_run = w.is_dry_run;
    let android = w
        .get_android_actor_mut(&format!("@{}", actor))
        .ok_or_else(|| format!("Actor @{} not found", actor))?;

    if is_dry_run {
        println!("DRY-RUN: Forwarding step to guest: {}", step);
        return Ok(());
    }

    android.execute_step(&step, 60).await.map_err(|e| e.to_string())?;
    Ok(())
}

#[verify_macros::step_module]
pub mod custom_steps {
    use verify_macros::step;

    use super::*;

    #[step(r#"@(\S+) says hello to "([^"]+)""#)]
    async fn android_says_hello(
        w: &mut TestContext,
        actor: String,
        name: String,
    ) -> Result<(), String> {
        execute_guest_step_helper(w, actor, name).await
    }

    #[step(r#"@(\S+) checks wifi is connected"#)]
    async fn android_checks_wifi_connected(
        w: &mut TestContext,
        actor: String,
    ) -> Result<(), String> {
        let step = "Android checks wifi is connected".to_string();
        if w.is_dry_run {
            println!("DRY-RUN: Forwarding step to guest: {}", step);
            return Ok(());
        }
        let android = w
            .get_android_actor_mut(&format!("@{}", actor))
            .ok_or_else(|| format!("Actor @{} not found", actor))?;
        android.execute_step(&step, 60).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> std::result::Result<(), String> {
    let cli = Cli::parse();

    let mut features = Features::<TestContext>::new();
    verify_host_lib::adb_steps::register_steps(&mut features);
    verify_host_lib::android_steps::register_steps(&mut features);
    custom_steps::register_steps(&mut features);

    orchestrator::run(cli.args, features).await?;

    Ok(())
}
