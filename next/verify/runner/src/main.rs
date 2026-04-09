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
    /// Run integration tests
    Run {
        #[arg(long, help = "Path to the Android SDK root, platform-tools, or adb binary")]
        android_home: Option<String>,
        #[arg(long, help = "Path to the ntest-agent APK")]
        apk_path: Option<String>,
        #[arg(long, help = "Path to netsimd binary")]
        netsim_path: Option<String>,
        #[arg(long, help = "Path to netsim CLI binary")]
        netsim_cli_path: Option<String>,
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
    },
    /// List available test scenarios
    Scenarios,
}

mod adb_steps;
mod android_steps;
mod host_steps;
mod netsim_link_steps;
mod netsim_steps;
mod orchestrator;
mod scenarios;
mod types;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Run {
            android_home,
            apk_path,
            netsim_path,
            netsim_cli_path,
            netsim_args,
            gateway_ip,
            filter,
            dry_run,
            verbose,
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

            // Orchestrate Android integration tests
            orchestrator::run_android(
                Some(resolved_android_home),
                netsim_path,
                netsim_cli_path,
                netsim_args,
                apk_path,
                gateway_ip,
                filter,
                dry_run,
                verbose,
            )
            .await?;
        }
        Commands::Scenarios => {
            orchestrator::list_scenarios().await;
        }
    }
    Ok(())
}
