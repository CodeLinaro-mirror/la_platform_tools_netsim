// Copyright 2023-2025 The Android Open Source Project

use crate::args::Args;
use crate::ini_file::{IniFile, IniFileAccess, IniFileGuard, NetsimConfig};
use crate::logger;
use crate::platform;
use devices::Server as DeviceServer;
use futures::{SinkExt, StreamExt};
use grpc_server::packet_streamer::PacketStreamerService;
use log::{error, info};
use netsim_api::chips::{
    BluetoothMode, BluetoothParams, CellParams, ChipConfig, DeviceParams, NetworkKind,
    NetworkParams, PacketSink as ApiPacketSink, PacketStream as ApiPacketStream, UwbParams,
    WifiParams,
};
use netsim_api::devices::{DeviceClient, DeviceConfig, DevicePsCreate};
use netsim_api::initial_info::{ChipInfo, ChipKind};
use packet_stream::transport::traits::{PacketSink, PacketStream};
use packet_stream::{StreamAddress, Streams, TransportType};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::task::JoinSet;

#[derive(Debug, PartialEq)]
pub enum RunResult {
    ExitedNormally,
    InitializationError(String),
}

fn init_error<E: std::fmt::Display>(e: E) -> RunResult {
    RunResult::InitializationError(e.to_string())
}

#[derive(Debug)]
pub enum StartUpMode {
    Owner(NetsimDaemon, IniFileGuard),
    Client(NetsimConfig),
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
    device_guid: String,
) {
    info!("Handling new connection for {:?}, device {}", chip_info.name, chip_info.device_name());
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
        ChipKind::BLUETOOTH => NetworkParams::Bluetooth(BluetoothParams {
            address: "".to_string(), // TODO: Get address from ChipInfo
            bt_properties: Default::default(),
            mode: BluetoothMode::Device(DeviceParams {}),
        }),
        ChipKind::UWB => NetworkParams::Uwb(UwbParams {}),
        ChipKind::WIFI => NetworkParams::Wifi(WifiParams {}),
        ChipKind::CELL => NetworkParams::Cell(CellParams {}),
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

    let request = DevicePsCreate {
        device_guid,
        packet_stream: Some(api_stream),
        packet_sink: Some(api_sink),
        device_config,
        chip_config,
    };

    if let Err(e) = device_client.ps_create(request).await {
        error!("Failed to register stream for {}: {}", chip_info.name, e);
    }
}

#[cfg(unix)]
async fn setup_uds_listener(
    streams: &mut Streams,
    listener_addresses: &mut HashMap<String, StreamAddress>,
    runtime_dir: &PathBuf,
) -> Result<(), RunResult> {
    let uds_path = runtime_dir.join("netsim.sock");
    if uds_path.exists() {
        fs::remove_file(&uds_path).map_err(init_error)?;
    }
    if let Some(parent) = uds_path.parent() {
        fs::create_dir_all(parent).map_err(init_error)?;
    }
    if let Some(uds_path_str) = uds_path.to_str() {
        streams
            .start_listener("netsim_uds", TransportType::uds(uds_path_str))
            .await
            .map_err(init_error)?;
        info!("Started UDS listener at {}", uds_path.display());
        if let Some(addr) = streams.listener_address("netsim_uds") {
            listener_addresses.insert("netsim_uds".to_string(), addr.clone());
        } else {
            return Err(RunResult::InitializationError(
                "Failed to get UDS listener address after creation".to_string(),
            ));
        }
        Ok(())
    } else {
        Err(RunResult::InitializationError(format!("Invalid UDS path: {}", uds_path.display())))
    }
}

async fn setup_grpc_listener(
    streams: &mut Streams,
    listener_addresses: &mut HashMap<String, StreamAddress>,
    requested_port: u16,
    device_client: DeviceClient,
) -> Result<(u16, grpcio::Server), RunResult> {
    // Create a channel to bridge PacketStreamerService connections to Streams
    let (new_connection_tx, new_connection_rx) = mpsc::channel(100);
    let packet_streamer_service = PacketStreamerService::new(new_connection_tx);

    // Start the gRPC server
    let (server, port) =
        grpc_server::server::start(requested_port.into(), device_client, packet_streamer_service)
            .map_err(|e| init_error(format!("Failed to start gRPC server: {}", e)))?;

    let listener = grpc_server::packet_streamer::ChannelTransportListener {
        rx: new_connection_rx,
        local_addr: StreamAddress::Grpc(std::net::SocketAddr::new(
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
            port,
        )),
    };

    let _ = streams.add_listener("netsim_grpc", Box::new(listener));

    info!("Started gRPC listener on port {}", port);
    listener_addresses.insert(
        "netsim_grpc".to_string(),
        StreamAddress::Grpc(std::net::SocketAddr::new(
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
            port,
        )),
    );

    Ok((port, server))
}

/// The main daemon for netsim-next.
///
/// This struct manages the lifecycle of various servers (Bluetooth, Device),
/// and handles incoming connections using the `packet_stream` crate.
#[derive(Debug)]
pub struct NetsimDaemon {
    /// Set of spawned Tokio tasks, including the chip and device servers.
    join_set: JoinSet<()>,
    /// Manages the network listeners (UDS, gRPC) for incoming connections.
    streams: Streams,
    /// Client for interacting with the Device Service.
    device_client: DeviceClient,
    /// Addresses of the active listeners.
    listener_addresses: HashMap<String, StreamAddress>,
    /// Command line arguments passed to the daemon.
    args: Args,
    /// The gRPC server instance (kept alive).
    _grpc_server: Option<grpcio::Server>,
}

impl NetsimDaemon {
    /// Creates a new `NetsimDaemon` instance or returns config for forwarding.
    ///
    /// Returns:
    /// - `Ok(StartUpMode::Owner)`: Daemon instance, lock acquired.
    /// - `Ok(StartUpMode::Client)`: Config of running daemon, lock not acquired.
    /// - `Err(RunResult::InitializationError)`: Fatal error.
    pub async fn new() -> Result<StartUpMode, RunResult> {
        let discovery_dir = crate::ini_file::get_discovery_directory();
        let runtime_dir = platform::get_runtime_dir();
        Self::new_with_dirs(discovery_dir, runtime_dir).await
    }

    /// Creates a new `NetsimDaemon` instance with custom directories.
    pub async fn new_with_dirs(
        discovery_dir: PathBuf,
        runtime_dir: PathBuf,
    ) -> Result<StartUpMode, RunResult> {
        #[cfg(all(target_os = "linux", feature = "cuttlefish"))]
        cuttlefish_init();

        logger::init("netsim", true);

        info!("netsim startup");

        let args = Args::parse();
        let mut ini_file = IniFile::new_for_dir(discovery_dir).map_err(init_error)?;

        // Attempt to acquire the singleton lock for the netsim daemon.
        // The lock file (netsim.ini.lock) is managed by the `named_lock` crate
        // in a system-wide temporary directory.
        match ini_file.try_acquire().map_err(init_error)? {
            // This instance is the Writer (the primary daemon).
            IniFileAccess::Writer(ini_guard) => {
                Self::initialize_primary_daemon(ini_guard, args, runtime_dir).await
            }
            // This instance is a Reader, another daemon is already running.
            IniFileAccess::Reader(config) => {
                info!("Lock held by another process. Reading config: {:?}", config);
                Ok(StartUpMode::Client(config))
            }
        }
    }

    async fn initialize_primary_daemon(
        ini_guard: IniFileGuard,
        args: Args,
        runtime_dir: PathBuf,
    ) -> Result<StartUpMode, RunResult> {
        info!("Successfully acquired lock. This instance is the Owner.");
        let ini_path = ini_guard.path();
        info!("INI file path: {}", ini_path.display());

        // Remove any potential stale INI file from a previous unclean shutdown.
        if ini_path.exists() {
            if let Err(e) = fs::remove_file(ini_path) {
                log::warn!("Failed to remove stale INI file: {}", e);
                // Continue anyway, as we will overwrite it
            }
        }

        // Initialize listeners (UDS, gRPC).
        let mut listener_addresses = HashMap::new();
        let mut streams = Streams::new();

        #[cfg(unix)]
        setup_uds_listener(&mut streams, &mut listener_addresses, &runtime_dir).await?;

        // Setup Device Server first to get the client
        let (device_server, device_client) = DeviceServer::new(args.no_shutdown);
        info!("Device server created");

        // gRPC port is determined after the listener starts.
        let (actual_grpc_port, grpc_server) = setup_grpc_listener(
            &mut streams,
            &mut listener_addresses,
            args.grpc_port.unwrap_or(0),
            device_client.clone(),
        )
        .await?;

        listener_addresses.insert(
            "netsim_grpc".to_string(),
            StreamAddress::Tcp(std::net::SocketAddr::from(([127, 0, 0, 1], actual_grpc_port))),
        );

        // Write the current daemon's information to the INI file.
        // Clients will use this to connect.
        let mut ini_data = HashMap::from([
            ("pid".to_string(), std::process::id().to_string()),
            ("grpc.port".to_string(), actual_grpc_port.to_string()),
        ]);
        if let Some(StreamAddress::Uds(path)) = listener_addresses.get("netsim_uds") {
            ini_data.insert("uds.path".to_string(), path.to_string_lossy().to_string());
        }

        // Even if stale file removal failed, we can proceed as ini_guard.write will overwrite.
        ini_guard.write(&ini_data).map_err(init_error)?;
        info!("Successfully wrote to INI file {}", ini_path.display());

        // Setup Bluetooth Server
        let (bt_server, bt_client) = bluetooth::Server::new(device_client.clone());
        info!("Bluetooth server created");

        // Setup Wifi Server
        let (wifi_server, wifi_client) = wifi::Server::new(device_client.clone());
        info!("Wifi server created");

        // Setup Uwb Server
        let (uwb_server, uwb_client) = uwb::Server::new(device_client.clone());
        info!("Uwb server created");

        // Setup Cell Server
        // TODO: Replace with real modem network.
        let cell_controller = cell::fake_modem_network::FakeModemNetwork::new();
        let (cell_server, cell_client) = cell::Server::new(device_client.clone(), cell_controller);
        info!("Cell server created");

        // Prepare chip clients map for DeviceServer
        let mut chip_clients = HashMap::new();
        chip_clients.insert(NetworkKind::Bluetooth, bt_client);
        chip_clients.insert(NetworkKind::Wifi, wifi_client);
        chip_clients.insert(NetworkKind::Uwb, uwb_client);
        chip_clients.insert(NetworkKind::Cell, cell_client);

        // Spawn server tasks
        let mut join_set = JoinSet::new();
        join_set.spawn(bt_server.run());
        info!("Bluetooth server started");
        join_set.spawn(wifi_server.run());
        info!("Wifi server started");
        join_set.spawn(uwb_server.run());
        info!("Uwb server started");
        join_set.spawn(cell_server.run());
        info!("Cell server started");
        join_set.spawn(device_server.run(chip_clients));
        info!("Device server started");
        Ok(StartUpMode::Owner(
            NetsimDaemon {
                join_set,
                streams,
                device_client,
                listener_addresses,
                args,
                _grpc_server: Some(grpc_server),
            },
            ini_guard,
        ))
    }

    /// Gets the path to the Unix Domain Socket, if one is active.
    pub fn uds_path(&self) -> Option<PathBuf> {
        self.listener_addresses.get("netsim_uds").and_then(|addr| match addr {
            StreamAddress::Uds(path) => Some(path.clone()),
            _ => None,
        })
    }

    /// Gets the gRPC port, if the server is running.
    pub fn grpc_port(&self) -> Option<u16> {
        self.listener_addresses.get("netsim_grpc").and_then(|addr| match addr {
            StreamAddress::Tcp(socket_addr) => Some(socket_addr.port()),
            _ => None,
        })
    }

    /// Runs the main event loop for the daemon.
    pub async fn run_daemon(mut self) {
        let dc = self.device_client.clone();
        let mut streams = self.streams;

        info!("Netsimd started {}", if self.args.no_shutdown { "--no-shutdown" } else { "" });

        loop {
            tokio::select! {
                // Branch 1: Wait for a new connection
                accept_result = streams.accept_any() => {
                    match accept_result {
                        Ok((listener_name, (stream, sink, chip_info, guid))) => {
                            info!(
                                "Accepted connection on {}: from {}",
                                listener_name,
                                chip_info.device_name()
                            );
                            let device_client = dc.clone();
                            // Await connection handling directly in the main loop
                            handle_new_connection(device_client, stream, sink, chip_info, guid).await;
                        }
                        Err(e) => {
                            error!("Error accepting connection: {}. Stopping new connections.", e);
                        }
                    }
                }

                // Branch 2: Wait for a task in the JoinSet to complete
                join_result = self.join_set.join_next() => {
                    match join_result {
                        Some(Ok(_)) => {
                            info!("A server task completed.");
                        }
                        Some(Err(e)) => {
                            error!("A server task panicked: {}", e);
                        }
                        None => {
                            info!("All server tasks in JoinSet have completed. Shutting down.");
                            break;
                        }
                    }
                }
            }
        }
        info!("NetsimDaemon main loop exited.");
    }
}

pub async fn run() -> RunResult {
    match NetsimDaemon::new().await {
        Ok(StartUpMode::Owner(daemon, _ini_guard)) => {
            daemon.run_daemon().await;
            RunResult::ExitedNormally
        }
        Ok(StartUpMode::Client(config)) => {
            info!("Another netsimd is running. Will use its config: {:?}", config);
            info!("Target gRPC port: {}", config.grpc_port);
            // TODO: Implement client/forwarder logic here for cuttlefish case
            RunResult::ExitedNormally // Placeholder
        }
        Err(e) => {
            error!("Failed to initialize NetsimDaemon: {:?}", e);
            e
        }
    }
}
