// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Command-line tool for running Netsim integration tests.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "runner")]
#[command(about = "Network Tester for Netsim orchestrator", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run e2e tests
    Run {
        #[arg(long, help = "Path to the Android SDK root, platform-tools, or adb binary")]
        android_home: Option<String>,
        #[arg(long, help = "Path to the vbs APK")]
        apk_path: Option<String>,
        #[arg(long, help = "Path to netsimd binary")]
        netsim_path: Option<String>,
        #[arg(long, help = "Arguments to pass to netsim")]
        netsim_args: Option<String>,
        #[arg(long, help = "Gateway IP to connect to (defaults to 10.0.2.2)")]
        gateway_ip: Option<String>,
        #[arg(long, help = "Optional scenario filter (matches feature file name)")]
        filter: Option<String>,
        #[arg(long, help = "Simulation mode (no-op for orchestrator logic verification)")]
        #[arg(default_value_t = false)]
        dry_run: bool,
        #[arg(long, short, help = "Enable verbose output")]
        verbose: bool,
        #[arg(long, help = "Directory containing feature files (specs)")]
        spec_dir: Option<String>,
        #[arg(long, help = "Continue running tests after a failure")]
        #[arg(default_value_t = false)]
        keep_going: bool,
    },
    /// List available test scenarios
    Scenarios {
        #[arg(long, help = "Directory containing feature files (specs)")]
        spec_dir: Option<String>,
    },
}

use verify_host_lib::{
    adb_steps, android_steps,
    features::Features,
    host_steps, netsim_link_steps, netsim_steps,
    orchestrator::{self, TestContext},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            android_home,
            apk_path,
            netsim_path,
            netsim_args,
            gateway_ip,
            filter,
            dry_run,
            verbose,
            spec_dir,
            keep_going,
        } => {
            let resolved_android_home = match android_home
                .or_else(|| std::env::var("ANDROID_HOME").ok())
            {
                Some(path) => path,
                None => {
                    anyhow::bail!(
                        "ANDROID_HOME environment variable is not set. Please set it to your Android SDK path or pass it via --android-home."
                    );
                }
            };
            let p = std::path::Path::new(&resolved_android_home);
            if !p.is_dir() {
                anyhow::bail!("ANDROID_HOME path is not a directory: {}", resolved_android_home);
            }
            if !p.join("platform-tools").join("adb").exists() && !p.join("adb").exists() {
                anyhow::bail!(
                    "ANDROID_HOME does not point to a valid Android SDK (missing adb). Path: {}",
                    resolved_android_home
                );
            }

            let mut features = Features::<TestContext>::new();
            adb_steps::register_steps(&mut features);
            android_steps::register_steps(&mut features);
            host_steps::register_steps(&mut features);
            netsim_steps::register_steps(&mut features);
            netsim_link_steps::register_steps(&mut features);

            // Orchestrate Android integration tests
            orchestrator::run_android(
                Some(resolved_android_home),
                netsim_path,
                netsim_args,
                apk_path,
                gateway_ip,
                filter,
                dry_run,
                verbose,
                features,
                spec_dir,
                keep_going,
            )
            .await
            .map_err(|e| anyhow::anyhow!(e))?;
        }
        Commands::Scenarios { spec_dir } => {
            let mut features = Features::<TestContext>::new();
            adb_steps::register_steps(&mut features);
            android_steps::register_steps(&mut features);
            host_steps::register_steps(&mut features);
            netsim_steps::register_steps(&mut features);
            netsim_link_steps::register_steps(&mut features);
            orchestrator::list_scenarios(features, spec_dir)
                .await
                .map_err(|e| anyhow::anyhow!(e))?;
        }
    }

    Ok(())
}
