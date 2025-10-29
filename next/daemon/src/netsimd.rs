// Copyright 2023-2025 The Android Open Source Project

use crate::config::IniFile;
use crate::logger;
use crate::platform;
use devices::Server as DeviceServer;
use futures::{SinkExt, StreamExt};
use log::{error, info};
use netsim_api::chips::{
    BluetoothMode, BluetoothParams, ChipConfig, DeviceParams, NetworkParams,
    PacketSink as ApiPacketSink, PacketStream as ApiPacketStream,
};
use netsim_api::devices::{CreateDeviceParams, DeviceClient, DeviceConfig};
use netsim_api::initial_info::{ChipInfo, ChipKind};
use packet_stream::transport::traits::{PacketSink, PacketStream};
use packet_stream::{StreamAddress, Streams, TransportType};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use tokio::task::JoinSet;

const INI_FILENAME: &str = "netsim-next.ini";

#[derive(Debug, PartialEq)]
pub enum RunResult {
    AlreadyRunning,
    ExitedNormally,
    InitializationError(String),
}

// Initialization for linux cuttlefish environment. Cuttelfish passes
// open file descriptors to netsimd.

#[cfg(all(target_os = "linux", feature = "cuttlefish"))]
fn cuttlefish_init() {
    use rustutils::inherited_fd;
    // SAFETY: This function must be called before any other code that might take ownership of
    // file descriptors. `init_once` takes ownership of all open file descriptors except for
    // the stdio streams. Calling it after other parts of the program has already acquired
    // ownership of file descriptors can lead to double-frees or other memory corruption issues.
    unsafe {
        inherited_fd::init_once().expect("inherited_fds");
    }
}

async fn handle_new_connection(
    device_client: DeviceClient,
    stream: PacketStream,
    sink: PacketSink,
    chip_info: ChipInfo,
) {
    info!("Handling new connection for {:?}, device {}", chip_info.name, chip_info.device_name());

    let device_guid =
        chip_info.device_info.as_ref().map_or("unknown_device".to_string(), |d| d.id.clone());
    let device_config = DeviceConfig {
        name: chip_info
            .device_info
            .as_ref()
            .map_or("Unknown Device".to_string(), |d| d.name.clone()),
        visible: true,
        position: Default::default(),
        orientation: Default::default(),
    };

    let chip = match chip_info.chip {
        Some(chip) => chip,
        None => {
            error!("ChipInfo missing chip details for {}", chip_info.name);
            return;
        }
    };

    let network_params = match chip.kind {
        ChipKind::Bluetooth => NetworkParams::Bluetooth(BluetoothParams {
            address: "".to_string(), // TODO: Get address from ChipInfo
            bt_properties: Default::default(),
            mode: BluetoothMode::Device(DeviceParams {}),
        }),
        _ => {
            error!("Unsupported chip kind: {:?}", chip.kind);
            return;
        }
    };

    let chip_config = ChipConfig {
        name: chip.name.clone(),
        manufacturer: chip.manufacturer.clone(),
        product_name: chip.product_name.clone(),
        network_params,
    };

    // Convert packet_stream types to netsim_api types
    let api_stream: ApiPacketStream = Box::new(stream.filter_map(|item| {
        Box::pin(async move {
            match item {
                Ok(bytes) => Some(bytes),
                Err(e) => {
                    error!("Error in packet stream: {}", e);
                    None
                }
            }
        })
    }));

    let api_sink: ApiPacketSink =
        Box::pin(sink.sink_map_err(|e| io::Error::new(io::ErrorKind::Other, e)));

    let params = CreateDeviceParams {
        device_guid,
        packet_stream: Some(api_stream),
        packet_sink: Some(api_sink),
        device_config,
        chip_config,
    };

    if let Err(e) = device_client.ps_create(params).await {
        error!("Failed to register stream for {}: {}", chip_info.name, e);
    }
}

/// The main daemon for netsim-next.
///
/// This struct manages the lifecycle of various servers (Bluetooth, Device),
/// and handles incoming connections using the `packet_stream` crate.
pub struct NetsimDaemon {
    join_set: JoinSet<()>,
    streams: Streams,
    device_client: DeviceClient,
    listener_addresses: HashMap<String, StreamAddress>,
}

impl NetsimDaemon {
    /// Creates a new `NetsimDaemon` instance.
    ///
    /// Initializes the logger, acquires a lock file to ensure a single instance,
    /// sets up listeners for UDS and TCP, and starts the Bluetooth and Device servers.
    ///
    /// Returns a `Result` with the `NetsimDaemon` or an error string if initialization fails.
    pub async fn new() -> Result<(Self, IniFile), RunResult> {
        #[cfg(all(target_os = "linux", feature = "cuttlefish"))]
        cuttlefish_init();

        logger::init("netsim", true);

        info!("netsim startup");

        let mut ini_file = IniFile::new(&platform::get_runtime_dir(), INI_FILENAME);

        // 1. Lock: Attempt to acquire the lock.
        ini_file.try_lock().map_err(|_e| {
            error!(
                "Failed to acquire lock on {}. Another instance may be running.",
                ini_file.path().display()
            );
            RunResult::AlreadyRunning
        })?;
        info!("Successfully acquired lock on {}", ini_file.path().display());

        let mut listener_addresses = HashMap::new();
        let mut streams = Streams::new();

        // Start UDS listener
        #[cfg(unix)]
        {
            let uds_path = platform::get_runtime_dir().join("netsim.sock");
            if uds_path.exists() {
                fs::remove_file(&uds_path)
                    .map_err(|e| RunResult::InitializationError(e.to_string()))?;
            }
            if let Some(uds_path_str) = uds_path.to_str() {
                streams
                    .start_listener("netsim_uds", TransportType::uds(uds_path_str))
                    .await
                    .map_err(|e| RunResult::InitializationError(e.to_string()))?;
                info!("Started UDS listener at {}", uds_path.display());
                listener_addresses.insert(
                    "netsim_uds".to_string(),
                    streams.listener_address("netsim_uds").unwrap().clone(),
                );
            } else {
                return Err(RunResult::InitializationError(format!(
                    "Invalid UDS path: {}",
                    uds_path.display()
                )));
            }
        }

        // TODO: Start TCP listener and add to listener_addresses

        // Setup Bluetooth Server
        let (bt_server, bt_client) = bluetooth::Server::new();
        let mut join_set = JoinSet::new();
        join_set.spawn(bt_server.run());
        info!("Bluetooth server started");

        // Setup Device Server
        let (device_server, device_client) = DeviceServer::new(bt_client);
        join_set.spawn(device_server.run());
        info!("Device server started");

        Ok((Self { join_set, streams, device_client, listener_addresses }, ini_file))
    }

    /// Gets the path to the Unix Domain Socket, if one is active.
    pub fn uds_path(&self) -> Option<PathBuf> {
        self.listener_addresses.get("netsim_uds").and_then(|addr| match addr {
            StreamAddress::Uds(path) => Some(path.clone()),
            _ => None,
        })
    }

    /// Runs the main event loop for the daemon.
    ///
    /// This function concurrently:
    /// 1. Listens for and accepts new connections on the configured transports.
    /// 2. Monitors the health and completion of the spawned server tasks (Bluetooth, Device).
    ///
    /// The loop terminates when all essential server tasks in the `JoinSet` have completed.
    pub async fn run(mut self) {
        let dc = self.device_client.clone();
        let mut streams = self.streams;

        loop {
            tokio::select! {
                // Branch 1: Wait for a new connection
                accept_result = streams.accept_any() => {
                    match accept_result {
                        Ok((listener_name, (stream, sink, chip_info))) => {
                            info!(
                                "Accepted connection on {}: from {}",
                                listener_name,
                                chip_info.device_name()
                            );
                            let device_client = dc.clone();
                            // Await connection handling directly in the main loop
                            handle_new_connection(device_client, stream, sink, chip_info).await;
                        }
                        Err(e) => {
                            error!("Error accepting connection: {}. Stopping new connections.", e);
                            // Optional: Could break here if accept errors are fatal to the whole daemon
                            // break;
                        }
                    }
                }

                // Branch 2: Wait for a task in the JoinSet to complete
                join_result = self.join_set.join_next() => {
                    match join_result {
                        Some(Ok(_)) => {
                            // A task completed successfully. Keep looping.
                            info!("A server task completed.");
                        }
                        Some(Err(e)) => {
                            error!("A server task panicked: {}", e);
                            // Optional: Decide if a panicking task should shut down the daemon
                            // break;
                        }
                        None => {
                            info!("All server tasks in JoinSet have completed. Shutting down.");
                            break; // Exit the main loop
                        }
                    }
                }
            }
        }
        info!("NetsimDaemon main loop exited.");

        let (bt_server, bt_client) = bluetooth::Server::new();
        let (devices_server, _devices_client) = devices::Server::new(bt_client);
        self.join_set.spawn(bt_server.run());
        info!("bluetooth server started");
        self.join_set.spawn(devices_server.run());
        info!("devices server started");
    }
}

/// Runs the netsim daemon.
///
/// This is the main entry point for starting the daemon. It creates and runs
/// the `NetsimDaemon` instance.
/// Returns `RunResult` indicating the outcome.
pub async fn run() -> RunResult {
    match NetsimDaemon::new().await {
        Ok((daemon, _ini_file)) => {
            // _ini_file is kept in scope to hold the lock
            daemon.run().await;
            RunResult::ExitedNormally
        }
        Err(e) => {
            error!("Failed to initialize NetsimDaemon: {:?}", e);
            e
        }
    }
}
