//! # Modem Simulator Main Entry Point
//!
//! This module is the main entry point for the `modem_simulator` binary.
//! See the project `README.md` for a full architectural overview.
//!
//! This module's sole responsibility is to:
//! - Parse command-line arguments.
//! - Initialize logging.
//! - Delegate control to the appropriate logic (client, server, or CLI).

use std::{collections::HashMap, process::Command, sync::Arc, time::Duration};

use clap::{Parser, Subcommand};
use log::{error, info};
use modem_rs::ModemId;
use serde::Deserialize;
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::Mutex,
};

mod client;
mod server;

const TCP_PORT: u16 = 5556;
const PID_FILE: &str = "/tmp/modem_central.pid";
const SERVER_LOG_FILE: &str = "/tmp/modem_central.log";

/// A command-line interface for the modem simulator.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    command: Option<CliCommand>,

    // These arguments are for the default "proxy" mode, which runs when no subcommand is given.
    /// A comma separated list of file descriptors. Required for proxy mode.
    #[arg(long)]
    server_fds: Option<String>,

    /// Sim type: 1 for normal, 2 for CtsCarrierApiTestCases
    #[arg(long, default_value_t = 1)]
    sim_type: u8,

    /// (Internal) Runs the application in server mode.
    #[arg(long, hide = true)]
    server_mode: bool,

    /// The instance ID of the AVD. Required for proxy mode.
    #[arg(long)]
    instance_id: Option<u32>,
}

#[derive(Subcommand, Debug)]
pub enum CliCommand {
    /// Command-line interface for injecting events into the simulator.
    Cli(CliArgs),
}

#[derive(Parser, Debug)]
pub struct CliArgs {
    #[command(subcommand)]
    command: CliSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum CliSubcommand {
    /// Send an SMS message to a simulated modem.
    SendSms {
        /// The instance ID of the target AVD.
        #[arg(long)]
        target_instance: u32,
        /// The local modem index on the target AVD (e.g., 0 for the first SIM).
        #[arg(long, default_value_t = 0)]
        target_modem: u64,
        /// The destination phone number.
        #[arg(long)]
        destination: String,
        /// The message content.
        #[arg(long)]
        message: String,
    },
    /// List all active modems and their statistics.
    ListModems,
}

#[derive(Deserialize, Debug)]
struct ModemInfo {
    instance_id: u32,
    local_modem_index: u64,
    global_modem_id: u64,
    // metrics: MetricsSnapshot,
}

type ClientWriter = Arc<Mutex<tokio::io::WriteHalf<TcpStream>>>;

struct ServerCallbacks {
    client_writers: Arc<Mutex<HashMap<ModemId, ClientWriter>>>,
}

/// Ensures the central server daemon is running, launching it if necessary.
async fn ensure_server_is_running() {
    if TcpStream::connect(format!("localhost:{}", TCP_PORT)).await.is_ok() {
        info!("[Main] Server is already running.");
        return;
    }

    info!("[Main] Server not running. Starting it now...");
    let exe = std::env::current_exe().expect("Failed to get current exe path");
    Command::new(exe)
        .arg("--server-mode")
        .arg("--server-fds") // Dummy arg, required by clap
        .arg("0")
        .spawn()
        .expect("Failed to start server process");

    for _ in 0..5 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if TcpStream::connect(format!("localhost:{}", TCP_PORT)).await.is_ok() {
            info!("[Main] Server started successfully.");
            return;
        }
    }

    error!("[Main] Failed to connect to server after starting it. Exiting.");
    std::process::exit(1);
}

async fn run_cli_command(args: CliArgs) {
    let stream = match TcpStream::connect(format!("localhost:{}", TCP_PORT)).await {
        Ok(stream) => stream,
        Err(e) => {
            error!("[CLI] Failed to connect to server: {}. Is it running?", e);
            return;
        }
    };

    let (mut reader, mut writer) = tokio::io::split(stream);

    match args.command {
        CliSubcommand::SendSms { target_instance, target_modem, destination, message } => {
            let global_modem_id = ((target_instance as u64) << 32) | target_modem;
            info!("[CLI] Sending SMS to modem_id {}", global_modem_id);

            let handshake = format!("INJECT?modem_id={}\r\n\r\n", global_modem_id);
            writer.write_all(handshake.as_bytes()).await.unwrap();

            let commands = vec![
                "AT+CMGF=1\r\n".to_string(),
                format!("AT+CMGS=\"{}\"\r\n", destination),
                format!("{}\x1A", message),
            ];

            let mut reader = BufReader::new(&mut reader);
            for cmd in commands {
                writer.write_all(cmd.as_bytes()).await.unwrap();
                let mut response = String::new();
                reader.read_line(&mut response).await.unwrap();
                print!("{}", response);
            }
            let mut response = String::new();
            reader.read_line(&mut response).await.unwrap();
            print!("{}", response);
        }
        CliSubcommand::ListModems => {
            let handshake = "GET /modems\r\n\r\n";
            writer.write_all(handshake.as_bytes()).await.unwrap();

            let mut response_buf = Vec::new();
            reader.read_to_end(&mut response_buf).await.unwrap();
            let response_str = String::from_utf8_lossy(&response_buf);

            if let Some(body) = response_str.split("\r\n\r\n").nth(1) {
                let modems: Vec<ModemInfo> = serde_json::from_str(body).unwrap_or_default();
                println!("{:<10} {:<7} {:<20}", "INSTANCE", "MODEM", "GLOBAL ID");
                for modem in modems {
                    println!(
                        "{:<10} {:<7} {:<20}",
                        modem.instance_id, modem.local_modem_index, modem.global_modem_id,
                    );
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    if std::env::args().any(|arg| arg == "--server-mode") {
        env_logger::init();
    } else if let Ok(val) = std::env::var("RUST_LOG") {
        if !val.is_empty() {
            env_logger::init();
        }
    }

    let args = Args::parse();

    if args.server_mode {
        server::run().await;
    } else if let Some(command) = args.command {
        match command {
            CliCommand::Cli(cli_args) => {
                run_cli_command(cli_args).await;
            }
        }
    } else {
        ensure_server_is_running().await;
        client::run(args).await;
    }
}

#[cfg(test)]
mod tests {
    use tokio::net::TcpListener;

    use super::*;

    #[test]
    fn test_parse_args_with_instance_id() {
        let args =
            Args::parse_from(&["modem_simulator", "--server-fds", "1,2", "--instance-id", "123"]);
        assert_eq!(args.instance_id, Some(123));
    }

    #[test]
    fn test_parse_args_without_instance_id() {
        let args = Args::parse_from(&["modem_simulator", "--server-fds", "1,2"]);
        assert_eq!(args.instance_id, None);
    }

    // #[tokio::test]
    // async fn test_ensure_server_is_running_when_not_running() {
    //     // Ensure no server is running on the port
    //     let _ = TcpListener::bind(format!("localhost:{}",
    // TCP_PORT)).await.unwrap();

    //     let exe = std::env::current_exe().expect("Failed to get current exe
    // path");     Command::new(exe).arg("--server-mode").spawn().expect("Failed
    // to start server process");

    //     ensure_server_is_running().await;
    // }

    #[tokio::test]
    async fn test_ensure_server_is_running_when_already_running() {
        let listener = TcpListener::bind(format!("localhost:{}", TCP_PORT)).await.unwrap();
        ensure_server_is_running().await;
        drop(listener);
    }
}
