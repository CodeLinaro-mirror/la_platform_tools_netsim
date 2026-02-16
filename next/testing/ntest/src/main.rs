//! Command-line tool for running Netsim integration tests.

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ntest")]
#[command(about = "Network Tester for Netsim", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Server mode (Reflector)
    Server {
        #[arg(long, default_value_t = 8080)]
        port: u16,
    },
    /// Client mode (Tester)
    Client {
        #[arg(long, default_value = "tcp")]
        proto: String,
        #[arg(long)]
        target: String,
        #[arg(long, default_value_t = 1024)]
        payload_size: usize,
        #[arg(long, default_value_t = false)]
        expect_eof: bool,
        #[arg(long, default_value_t = 1000)]
        timeout: u64,
    },
    /// Run tests locally (Self-Test)
    Local {
        #[arg(long, help = "Use specific TAP interface pattern")]
        wifi_tap: Option<String>,
        #[arg(long, help = "Use Cuttlefish TAP pool (implies 192.168.96.1)")]
        wifi_cvd_tap: bool,
        #[arg(long, help = "Gateway IP to connect to (overrides defaults)")]
        gateway_ip: Option<String>,
    },
    /// Run tests on Android (Integration Test)
    Android {
        #[arg(
            long,
            help = "Path to the android binary to push (e.g. ./target/aarch64-linux-android/debug/ntest)"
        )]
        android_bin: Option<String>,
        #[arg(long, help = "Path to netsim binary")]
        netsim_bin: Option<String>,
        #[arg(long, help = "Arguments to pass to netsim")]
        netsim_args: Option<String>,
    },
    /// List available test scenarios
    Scenarios,
}

mod client;
mod common;
mod server;
mod tests;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Server { port } => {
            server::run_server(port).await?;
        }
        Commands::Client { proto, target, payload_size, expect_eof, timeout } => {
            client::run_client(proto, target, payload_size, expect_eof, timeout).await?;
        }
        Commands::Local { wifi_tap, wifi_cvd_tap, gateway_ip } => {
            tests::run_local(wifi_tap, wifi_cvd_tap, gateway_ip).await?;
        }
        Commands::Android { android_bin, netsim_bin, netsim_args } => {
            tests::run_android(android_bin, netsim_bin, netsim_args).await?;
        }
        Commands::Scenarios => {
            tests::list_scenarios();
        }
    }
    Ok(())
}
