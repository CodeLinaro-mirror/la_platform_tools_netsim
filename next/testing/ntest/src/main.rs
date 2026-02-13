//! Command-line tool for running Netsim integration tests.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ntest")]
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
        #[arg(long, help = "Path to netsim binary")]
        netsim_path: Option<String>,
        #[arg(long, help = "Arguments to pass to netsim")]
        netsim_args: Option<String>,
        #[arg(long, help = "Gateway IP to connect to (defaults to 10.0.2.2)")]
        gateway_ip: Option<String>,
        #[arg(long, help = "Simulation mode (no-op for orchestrator logic verification)")]
        dry_run: bool,
    },
    /// List available test scenarios
    Scenarios,
}

mod common;
mod server;
mod tests;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Run { android_home, apk_path, netsim_path, netsim_args, gateway_ip, dry_run } => {
            // Orchestrate Android integration tests
            tests::run_android(
                android_home,
                netsim_path,
                netsim_args,
                apk_path,
                gateway_ip,
                dry_run,
            )
            .await?;
        }
        Commands::Scenarios => {
            tests::list_scenarios().await;
        }
    }
    Ok(())
}
