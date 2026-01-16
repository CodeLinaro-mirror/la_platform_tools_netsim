// Copyright 2023-2025 The Android Open Source Project // touch

use crate::args::Args;
use crate::ini_file::{IniFile, IniFileAccess, IniFileGuard, NetsimConfig};
use crate::logger;
use crate::platform;
use client::{CaptureClient, DeviceClient};
use common::system::netsimd_temp_dir;
use common::util::os_utils::{get_instance_name, redirect_std_stream};
use device_api::{DeviceAddChip, DeviceConfig};
use futures::{SinkExt, StreamExt};
use grpc_server::packet_streamer::PacketStreamerService;
use log::{error, info, warn};
use netsim_model::chip::{
    BluetoothCreate, BluetoothMode, CellCreate, ChipConfig, DeviceParams, NetworkKind,
    NetworkParams, PacketSink as ApiPacketSink, PacketStream as ApiPacketStream, UwbCreate,
    WifiCreate,
};
use netsim_model::initial_info::{ChipInfo, ChipKind};
use packet_stream::transport::traits::{PacketSink, PacketStream};
use packet_stream::{StreamAddress, Streams, TransportType};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;
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
    _capture_client: CaptureClient,
    _next_chip_id: Arc<AtomicU32>,
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

    let mut chip = match chip_info.chip {
        Some(chip) => chip,
        None => {
            warn!("ChipInfo missing 'chip' field for {}. Dropping connection.", chip_info.name);
            return;
        }
    };

    if let Some(device_info) = chip_info.device_info.as_ref().filter(|d| !d.avd_path.is_empty()) {
        chip.address =
            crate::avd_config::resolve_bluetooth_mac(&device_info.avd_path, &chip.address);
    }

    if chip.address.is_empty() && chip.id.len() == 17 {
        chip.address = chip.id.clone();
    }

    let network_params = match chip.kind {
        ChipKind::BLUETOOTH => NetworkParams::Bluetooth(BluetoothCreate {
            address: chip.address.clone(),
            bt_properties: Default::default(),
            mode: BluetoothMode::Device(DeviceParams {}),
        }),
        ChipKind::UWB => NetworkParams::Uwb(UwbCreate::default()),
        ChipKind::WIFI => NetworkParams::Wifi(WifiCreate::default()),
        ChipKind::CELL => NetworkParams::Cell(CellCreate::default()),
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

    // Convert packet_stream types to netsim_model types
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

    let request = DeviceAddChip {
        device_guid,
        packet_stream: Some(api_stream),
        packet_sink: Some(api_sink),
        device_config,
        chip_config,
    };

    if let Err(e) = device_client.add_chip(request).await {
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
        info!("UDS listener: {}", uds_path.display());
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
    link_client: client::LinkClient,
) -> Result<(u16, grpcio::Server), RunResult> {
    // Create a channel to bridge PacketStreamerService connections to Streams
    let (new_connection_tx, new_connection_rx) = mpsc::channel(100);
    let packet_streamer_service = PacketStreamerService::new(new_connection_tx);

    // Start the gRPC server
    let (server, port) = grpc_server::server::start(
        requested_port.into(),
        device_client,
        link_client,
        packet_streamer_service,
    )
    .map_err(|e| init_error(format!("Failed to start gRPC server: {}", e)))?;

    let listener = grpc_server::packet_streamer::ChannelTransportListener {
        rx: new_connection_rx,
        local_addr: StreamAddress::Grpc(std::net::SocketAddr::new(
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(0, 0, 0, 0)),
            port,
        )),
    };

    let _ = streams.add_listener("netsim_grpc", Box::new(listener));

    info!("gRPC port: {}", port);
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
    /// Client for interacting with the Capture Service.
    capture_client: CaptureClient,
    /// Shared counter for generating ChipIds.
    next_chip_id: Arc<AtomicU32>,
    /// Addresses of the active listeners.
    listener_addresses: HashMap<String, StreamAddress>,
    /// Command line arguments passed to the daemon.
    args: Args,
    /// The gRPC server instance (kept alive).
    _grpc_server: Option<grpcio::Server>,
}

impl NetsimDaemon {
    /// Returns a reference to the DeviceClient.
    pub fn device_client(&self) -> &DeviceClient {
        &self.device_client
    }

    /// Creates a new `NetsimDaemon` instance or returns config for forwarding.
    ///
    /// Returns:
    /// - `Ok(StartUpMode::Owner)`: Daemon instance, lock acquired.
    /// - `Ok(StartUpMode::Client)`: Config of running daemon, lock not acquired.
    /// - `Err(RunResult::InitializationError)`: Fatal error.
    pub async fn new() -> Result<StartUpMode, RunResult> {
        let discovery_dir = crate::ini_file::get_discovery_directory();
        let runtime_dir = platform::get_runtime_dir();
        Self::new_with_dirs(discovery_dir, runtime_dir, Args::parse()).await
    }

    /// Creates a new `NetsimDaemon` instance with custom directories.
    pub async fn new_with_dirs(
        discovery_dir: PathBuf,
        runtime_dir: PathBuf,
        args: Args,
    ) -> Result<StartUpMode, RunResult> {
        #[cfg(all(target_os = "linux", feature = "cuttlefish"))]
        cuttlefish_init();

        logger::init("netsim", true);

        info!("netsim startup");

        // enable Rust backtrace by setting env RUST_BACKTRACE=full
        env::set_var("RUST_BACKTRACE", "full");

        // Log where netsim artifacts are located
        info!("Artifacts: {:?}", netsimd_temp_dir());
        // Log all args
        info!("{args:#?}");

        if !args.logtostderr {
            if let Err(err) =
                redirect_std_stream(&get_instance_name(args.instance, args.connector_instance))
            {
                error!("{err:?}");
            }

            // Duplicating the previous two logs to be included in netsim_stderr.log
            info!("netsim artifacts path: {:?}", netsimd_temp_dir());
            info!("{args:#?}");
        }

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
        info!("Acquired lock (Owner)");
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

        // Setup Link Server
        let (link_runner, link_client) = link_actor::new();

        // Setup Device Server Channel
        let (device_runner, device_client) = device_actor::new();

        // Setup Capture Server
        let (capture_runner, capture_generic_client) = capture_actor::new();
        let capture_client = client::CaptureClient::new(capture_generic_client);

        let next_chip_id = Arc::new(AtomicU32::new(0));

        // gRPC port is determined after the listener starts.
        let (actual_grpc_port, grpc_server) = setup_grpc_listener(
            &mut streams,
            &mut listener_addresses,
            args.grpc_port.unwrap_or(0),
            device_client.clone(),
            link_client.clone(),
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
        info!("Wrote to INI file {}", ini_path.display());

        // Setup Bluetooth Server
        let (bt_runner, bt_client) = bluetooth_actor::new();
        let bt_actor_state =
            bluetooth_actor::BluetoothActor::new(device_client.clone(), bt_client.clone());

        // Setup Wifi Server
        let (wifi_server, wifi_client) = wifi::Server::new(device_client.clone());

        // Setup Uwb Server
        let (uwb_runner, uwb_client) = uwb::new();
        let uwb_actor = uwb::UwbActor::new(device_client.clone());

        // Setup Cell Server
        // TODO: Replace with real modem network.
        let cell_controller = cell::fake_modem_network::FakeModemNetwork::new();
        let (cell_runner, cell_client) = cell::new();
        let cell_server = cell::Server::new(device_client.clone(), cell_controller);

        // Prepare chip clients map for DeviceServer
        let mut chip_clients: HashMap<NetworkKind, Box<dyn netsim_model::chip::ChipClient>> =
            HashMap::new();
        chip_clients.insert(NetworkKind::Bluetooth, Box::new(bt_client.clone()));
        chip_clients.insert(NetworkKind::Wifi, Box::new(wifi_client.clone()));
        chip_clients.insert(NetworkKind::Uwb, Box::new(uwb_client.clone()));
        chip_clients.insert(NetworkKind::Cell, Box::new(cell_client.clone()));

        // Setup Link Actor State
        // Create a new map for LinkActor.
        // We need to inject ChipClients into LinkActor so it can propagate link changes
        // (like RSSI updates) to the underlying radio actors (e.g., BluetoothActor).
        let link_chip_clients = chip_clients.iter().map(|(&k, v)| (k.into(), v.clone())).collect();
        let link_actor_state = link_actor::LinkActor::new(link_chip_clients);

        let device_actor_state = device_actor::DeviceActor::new(
            chip_clients,
            next_chip_id.clone(),
            Some(Arc::new(capture_client.clone())),
            Box::new(link_client.clone()),
            None,
            Some(std::time::Duration::from_secs(15)),
        );

        // Spawn server tasks
        let mut join_set = JoinSet::new();
        join_set.spawn(bt_runner.run(bt_actor_state));
        info!("Bluetooth server started");
        join_set.spawn(wifi_server.run());
        info!("Wifi server started");
        join_set.spawn(uwb_runner.run(uwb_actor));
        info!("Uwb server started");
        join_set.spawn(cell_runner.run(cell_server));
        info!("Cell server started");
        join_set.spawn(device_runner.run(device_actor_state));
        info!("Device server started");
        join_set.spawn(capture_runner.run(capture_actor::CaptureActor::default()));
        info!("Capture server started");
        if args.pcap {
            capture_client.set_default_capture(true).await.expect("Failed to set default capture");
        }
        join_set.spawn(link_runner.run(link_actor_state));
        info!("Link server started");
        Ok(StartUpMode::Owner(
            NetsimDaemon {
                join_set,
                streams,
                device_client,
                capture_client,
                next_chip_id,
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
        let cc = self.capture_client.clone();
        let nci = self.next_chip_id.clone();
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
                            let capture_client = cc.clone();
                            let next_chip_id = nci.clone();
                            // Spawn connection handling to avoid blocking the main loop
                            // handle_new_connection performs async operations (like device_client.add_chip)
                            // which could delay accepting other connections if awaited directly.
                            tokio::spawn(handle_new_connection(device_client, capture_client, next_chip_id, stream, sink, chip_info, guid));
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
