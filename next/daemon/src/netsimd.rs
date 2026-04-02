// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    env, io,
    path::PathBuf,
    sync::{atomic::AtomicU32, Arc},
    time::Duration,
};

use capture_actor::CaptureClient;
use common::{
    system::netsimd_temp_dir,
    util::os_utils::{
        get_discovery_directory, get_hci_port, get_instance, get_instance_name, redirect_std_stream,
    },
};
use device_actor::DeviceClient;
use device_api::{DeviceAddChip, DeviceConfig};
use futures::{SinkExt, StreamExt};
use grpc_server::packet_streamer::PacketStreamerService;
use link_actor::LinkClient;
use netsim_model::{
    chip::{
        BluetoothCreate, BluetoothMode, CellCreate, ChipClient, ChipConfig, ChipKind,
        ChipKindParams, DeviceParams, PacketSink as ApiPacketSink, PacketStream as ApiPacketStream,
        UwbCreate, WifiCreate,
    },
    device::Pose,
    initial_info::ChipInfo,
    set_if_some,
};
use packet_stream::{
    transport::traits::{PacketSink, PacketStream},
    StreamAddress, Streams,
};
use slirp_actor::SlirpClient;
use tokio::{sync::mpsc, task::JoinSet};
use tracing::{error, info, warn};

use crate::{
    args::Args,
    ini_file::{IniFile, IniFileAccess, IniFileGuard, NetsimConfig},
    logger,
    version::get_version,
};

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
    // SAFETY: This function must be called before any other code that might take
    // ownership of file descriptors. `init_once` takes ownership of all open
    // file descriptors except for the stdio streams. Calling it after other
    // parts of the program has already acquired ownership of file descriptors
    // can lead to double-frees or other memory corruption issues.
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
        pose: Pose { position: Default::default(), orientation: Default::default() },
        builtin: false,
        device_info: chip_info.device_info.clone().map(Into::into),
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

    let chip_kind_params = match ChipKind::from(chip.kind) {
        ChipKind::BLUETOOTH => ChipKindParams::Bluetooth(BluetoothCreate {
            address: chip.address.clone(),
            bt_properties: Default::default(),
            mode: BluetoothMode::Device(DeviceParams {}),
        }),
        ChipKind::UWB => ChipKindParams::Uwb(UwbCreate::default()),
        ChipKind::WIFI => ChipKindParams::Wifi(WifiCreate::default()),
        ChipKind::CELLULAR => ChipKindParams::Cell(CellCreate::default()),
        kind => {
            error!("Unsupported chip kind: {:?}", kind);
            return;
        }
    };

    let chip_config = ChipConfig {
        name: chip.name.clone(),
        manufacturer: chip.manufacturer.clone(),
        product_name: chip.product_name.clone(),
        chip_kind_params,
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

async fn setup_grpc_listener(
    streams: &mut Streams,
    listener_addresses: &mut HashMap<String, StreamAddress>,
    requested_port: u16,
    enable_cli_ui: bool,
    device_client: DeviceClient,
    link_client: LinkClient,
    ap_client: ap_actor::ApClient,
    version: String,
) -> Result<(u16, grpcio::Server), RunResult> {
    // Create a channel to bridge PacketStreamerService connections to Streams
    let (new_connection_tx, new_connection_rx) = mpsc::channel(100);
    let packet_streamer_service = PacketStreamerService::new(new_connection_tx);

    // Start the gRPC server
    let (server, port) = grpc_server::server::start(
        requested_port.into(),
        enable_cli_ui,
        device_client,
        link_client,
        ap_client,
        packet_streamer_service,
        version,
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

/// The main daemon for netsim.
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
    /// The DeviceActor task handle.
    device_task: tokio::task::JoinHandle<()>,
    link_client: Box<dyn link_api::LinkClient>,

    slirp_client: Option<SlirpClient>,
    chip_clients: HashMap<ChipKind, Box<dyn ChipClient>>,
}

impl NetsimDaemon {
    /// Returns a reference to the DeviceClient.
    pub fn device_client(&self) -> &DeviceClient {
        &self.device_client
    }

    /// Returns a reference to the CaptureClient.
    pub fn capture_client(&self) -> &CaptureClient {
        &self.capture_client
    }

    /// Creates a new `NetsimDaemon` instance or returns config for forwarding.
    ///
    /// Returns:
    /// - `Ok(StartUpMode::Owner)`: Daemon instance, lock acquired.
    /// - `Ok(StartUpMode::Client)`: Config of running daemon, lock not
    ///   acquired.
    /// - `Err(RunResult::InitializationError)`: Fatal error.
    pub async fn new() -> Result<StartUpMode, RunResult> {
        let discovery_dir = get_discovery_directory();
        Self::new_with_dirs(discovery_dir, Args::parse()).await
    }

    /// Creates a new `NetsimDaemon` instance with custom directories.
    pub async fn new_with_dirs(
        discovery_dir: PathBuf,
        args: Args,
    ) -> Result<StartUpMode, RunResult> {
        if args.version {
            println!("Netsim version: {}", get_version());
            return Err(RunResult::ExitedNormally);
        }

        #[cfg(all(target_os = "linux", feature = "cuttlefish"))]
        cuttlefish_init();

        logger::init("netsim", args.verbose);

        info!("netsim startup");

        // enable Rust backtrace by setting env RUST_BACKTRACE=full
        env::set_var("RUST_BACKTRACE", "full");

        // Log where netsim artifacts are located
        info!("Artifacts: {:?}", netsimd_temp_dir());
        // Log all args
        info!("{args:#?}");

        // Resolve TAP configuration early to validate permissions/availability.
        #[cfg(target_os = "linux")]
        let wifi_tap = args.wifi.wifi_tap.clone().or_else(|| {
            if args.wifi.wifi_cvd_tap {
                Some("cvd-etap-%02d".to_string())
            } else {
                None
            }
        });

        // Pre-check TAP permissions if configured.
        // We do this BEFORE redirection so the user can see the error in the console.
        #[cfg(target_os = "linux")]
        if let Some(ref tap_config) = wifi_tap {
            if let Err(e) = wifi_actor::tap_gateway::TapGateway::preflight_check(tap_config) {
                return Err(RunResult::InitializationError(format!(
                    "TAP configuration failed: {}",
                    e
                )));
            }
        }

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

        info!(
            "Netsim Version: {}, OS: {}, Arch: {}",
            get_version(),
            std::env::consts::OS,
            std::env::consts::ARCH
        );

        let ini_file = IniFile::new_for_dir(discovery_dir).map_err(init_error)?;

        // Attempt to acquire the singleton lock for the netsim daemon.
        // The lock is managed by direct file locking on the netsim.ini file.
        match ini_file.try_acquire().map_err(init_error)? {
            // This instance is the Writer (the primary daemon).
            IniFileAccess::Writer(ini_guard) => {
                Self::initialize_primary_daemon(ini_guard, args).await
            }
            // This instance is a Reader, another daemon is already running.
            IniFileAccess::Reader(config) => {
                info!("Lock held by another process. Reading config: {:?}", config);
                Ok(StartUpMode::Client(config))
            }
        }
    }

    async fn initialize_primary_daemon(
        mut ini_guard: IniFileGuard,
        args: Args,
    ) -> Result<StartUpMode, RunResult> {
        info!("Acquired lock (Owner)");
        info!("INI file path: {}", ini_guard.path().display());

        // Initialize listeners (UDS, gRPC).
        let mut listener_addresses = HashMap::new();
        let mut streams = Streams::new();

        // Setup Link Server
        let (link_runner, link_client) = link_actor::new();

        // Setup Device Server Channel
        let (device_runner, device_client) = device_actor::new();

        // Setup Capture Server
        let (capture_runner, capture_client) = capture_actor::new();

        // Coordinate IDs across all actors
        let next_chip_id = Arc::new(AtomicU32::new(0));

        // Setup AP Actor (Needed for gRPC)
        let shared_keys = Arc::new(ap_actor::SharedKeyStore::new());
        let (ap_runner, ap_client) = ap_actor::new();
        let ap_actor_state = ap_actor::ApActor::new(shared_keys.clone());

        // gRPC port is determined after the listener starts.
        let (actual_grpc_port, grpc_server) = setup_grpc_listener(
            &mut streams,
            &mut listener_addresses,
            args.grpc_port.unwrap_or(0),
            !args.no_cli_ui,
            device_client.clone(),
            link_client.clone(),
            ap_client.clone(),
            get_version(),
        )
        .await?;

        listener_addresses.insert(
            "netsim_grpc".to_string(),
            StreamAddress::Tcp(std::net::SocketAddr::from(([127, 0, 0, 1], actual_grpc_port))),
        );

        // HCI TCP socket server
        let instance_num = get_instance(args.instance);
        let hci_port = args.hci_port.unwrap_or_else(|| get_hci_port(0, instance_num - 1) as u16);
        tokio::spawn(hci_server::server::run(hci_port, device_client.clone()));

        // WebSocket server
        let websocket_port = args.ws_port.map(|p| p + instance_num - 1);
        if let Some(ws_port) = websocket_port {
            tokio::spawn(websocket_server::server::run(ws_port, device_client.clone()));
        }

        // Write the current daemon's information to the INI file.
        // Clients will use this to connect.
        let mut ini_data = HashMap::from([
            ("pid".to_string(), std::process::id().to_string()),
            ("grpc.port".to_string(), actual_grpc_port.to_string()),
            ("hci.port".to_string(), hci_port.to_string()),
        ]);
        if let Some(ws_port) = websocket_port {
            ini_data.insert("ws.port".to_string(), ws_port.to_string());
        }
        if let Some(StreamAddress::Uds(path)) = listener_addresses.get("netsim_uds") {
            ini_data.insert("uds.path".to_string(), path.to_string_lossy().to_string());
        }

        // Even if stale file removal failed, we can proceed as ini_guard.write will
        // overwrite.
        ini_guard.write(&ini_data).map_err(init_error)?;
        info!("Wrote to INI file {}", ini_guard.path().display());

        // Setup Bluetooth Server
        let (bt_runner, bt_client) = bluetooth_actor::new();
        let bt_actor_state = bluetooth_actor::BluetoothActor::new(device_client.clone());

        // Setup Wifi Server (and dependencies: AP)
        // Setup Slirp Actor
        let (slirp_runner, slirp_client) = slirp_actor::new();
        let slirp_actor_state = slirp_actor::SlirpActor::new(
            Default::default(),
            args.http_proxy.clone(),
            args.host_dns.clone(),
        )
        .await;

        // (AP Actor already initialized above)

        // Setup Wifi Actor
        let (wifi_runner, wifi_client) = wifi_actor::new();
        // Initialize wifi_tap configuration.
        // If --wifi-cvd-tap is set, it implies explicit "cvd-etap-%02d" pattern for
        // pooling. If --wifi-tap is set, it overrides everything.
        #[cfg(target_os = "linux")]
        let wifi_tap = args.wifi.wifi_tap.clone().or_else(|| {
            if args.wifi.wifi_cvd_tap {
                Some("cvd-etap-%02d".to_string())
            } else {
                None
            }
        });
        #[cfg(not(target_os = "linux"))]
        let wifi_tap: Option<String> = None;

        // TAP preflight check is now done in `new_with_dirs` before lock acquisition.

        let wifi_actor_state = wifi_actor::WifiActor::new(
            Some(Arc::new(ap_client.clone())),
            Some(slirp_client.clone()),
            device_client.clone(),
            wifi_tap,
            shared_keys.clone(),
            Arc::new(wifi_actor::stats::SystemClock),
            args.forward_host_mdns,
        );

        // Setup Uwb Server
        let (uwb_runner, uwb_client) = uwb_actor::new();
        let uwb_actor = uwb_actor::UwbActor::new(device_client.clone());

        // Setup Cell Server
        let (cell_runner, cell_client) = cell_actor::new();
        let cell_actor_state = cell_actor::CellActor::new(device_client.clone());

        // Prepare chip clients map for DeviceServer
        let mut chip_clients: HashMap<ChipKind, Box<dyn ChipClient>> = HashMap::new();
        chip_clients.insert(ChipKind::BLUETOOTH, Box::new(bt_client.clone()));
        chip_clients.insert(ChipKind::WIFI, Box::new(wifi_client.clone()));
        chip_clients.insert(ChipKind::UWB, Box::new(uwb_client.clone()));
        chip_clients.insert(ChipKind::CELLULAR, Box::new(cell_client.clone()));
        // Note: ApClient is NOT added to chip_clients as it is now an independent
        // specialist.

        // Setup Link Actor State
        // Create a new map for LinkActor.
        // We need to inject ChipClients into LinkActor so it can propagate link changes
        // (like RSSI updates) to the underlying radio actors (e.g., BluetoothActor).
        let link_chip_clients = chip_clients.iter().map(|(&k, v)| (k, v.clone())).collect();
        let link_actor_state = link_actor::LinkActor::new(link_chip_clients);

        let (startup_timeout, idle_timeout) = if args.no_shutdown {
            (None, None)
        } else {
            (
                Some(args.startup_timeout.map_or(Duration::from_secs(15), Duration::from_millis)),
                Some(
                    args.idle_shutdown_timeout
                        .map_or(Duration::from_secs(0), Duration::from_millis),
                ),
            )
        };

        let mut device_actor_state = device_actor::DeviceActor::new(
            chip_clients.clone(),
            next_chip_id.clone(),
            Some(Arc::new(capture_client.clone())),
            Box::new(link_client.clone()),
            startup_timeout,
            idle_timeout,
            get_version(),
            None,
            None,
        );

        // Spawn server tasks
        let mut join_set = JoinSet::new();
        join_set.spawn(bt_runner.run(bt_actor_state));
        join_set.spawn(wifi_runner.run(wifi_actor_state));
        join_set.spawn(ap_runner.run(ap_actor_state));
        join_set.spawn(slirp_runner.run(slirp_actor_state));
        join_set.spawn(cell_runner.run(cell_actor_state));
        join_set.spawn(link_runner.run(link_actor_state));
        join_set.spawn(capture_runner.run(capture_actor::CaptureActor::new(args.pcap, None)));
        join_set.spawn(uwb_runner.run(uwb_actor));

        // Spawn DeviceActor separately
        device_actor_state.set_self_client(device_client.clone());
        let device_task = tokio::spawn(device_runner.run(device_actor_state));

        // Create Default AP
        let mut ap_config = ap_actor::ApConfig::default();
        if let Some(ssid) = &args.wifi.wifi_ssid {
            ap_config.ssid = ssid.clone();
        }
        set_if_some!(ap_config.wpa_passphrase, args.wifi.wifi_password.clone(), Some);
        set_if_some!(ap_config.channel, args.wifi.wifi_channel);
        set_if_some!(ap_config.beacon_interval, args.wifi.wifi_beacon_interval);
        set_if_some!(ap_config.hw_mode, args.wifi.wifi_mode, Into::into);

        ap_client.create_ap(Some(0), ap_config).await.expect("Failed to create default AP");

        // Clone chip_clients for NetsimDaemon
        let daemon_chip_clients = chip_clients.iter().map(|(k, v)| (*k, v.clone_box())).collect();

        Ok(StartUpMode::Owner(
            NetsimDaemon {
                device_client,
                capture_client,
                next_chip_id,
                join_set,
                streams,
                listener_addresses,
                args,
                _grpc_server: Some(grpc_server),
                device_task,
                link_client: Box::new(link_client),
                slirp_client: Some(slirp_client.clone()),
                chip_clients: daemon_chip_clients,
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

    async fn shutdown_actors(&mut self) {
        info!("Graceful shutdown requested for all actors");

        let link_fut = self.link_client.shutdown();
        let slirp_fut = async {
            if let Some(slirp) = &self.slirp_client {
                if let Err(e) = slirp.shutdown().await {
                    warn!("SlirpActor shutdown error: {}", e);
                }
            }
        };
        let chips_fut = futures::future::join_all(self.chip_clients.values().map(|c| c.shutdown()));

        // Execute all shutdown dispatches concurrently
        let (link_res, _, chips_res) = tokio::join!(link_fut, slirp_fut, chips_fut);

        if let Err(e) = link_res {
            warn!("LinkActor shutdown error: {}", e);
        }
        for res in chips_res {
            if let Err(e) = res {
                warn!("ChipActor shutdown error: {}", e);
            }
        }
    }

    async fn handle_device_actor_completion(&mut self, result: Result<(), tokio::task::JoinError>) {
        info!("DeviceActor exited. Shutting down daemon.");
        if let Err(e) = result {
            error!("DeviceActor panicked: {}", e);
        }
        self.shutdown_actors().await;
    }

    fn handle_secondary_task_completion(
        &mut self,
        result: Option<Result<(), tokio::task::JoinError>>,
    ) -> bool {
        match result {
            Some(Ok(_)) => {
                info!("A secondary server task completed. Shutting down.");
                true
            }
            Some(Err(e)) => {
                error!("A secondary server task panicked: {}", e);
                true
            }
            None => {
                // Should not happen as we have multiple tasks
                true
            }
        }
    }

    /// Runs the main event loop for the daemon.
    pub async fn run_daemon(mut self) -> RunResult {
        info!("Netsimd started {}", if self.args.no_shutdown { "--no-shutdown" } else { "" });

        loop {
            tokio::select! {
                // Branch 1: Handle incoming gRPC/UDS streams (New Clients)
                accept_result = self.streams.accept_any() => {
                    match accept_result {
                        Ok((listener_name, (stream, sink, chip_info, guid))) => {
                            info!("Accepted connection on {}:", listener_name);
                            // Spawning the handler ensures the main loop isn't blocked
                            let device_client = self.device_client.clone();
                            let capture_client = self.capture_client.clone();
                            let next_chip_id = self.next_chip_id.clone();
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

                // Branch 2: Wait for DeviceActor to complete (primary shutdown signal)
                device_result = &mut self.device_task => {
                    self.handle_device_actor_completion(device_result).await;
                    break;
                }

                // Branch 3: Wait for a task in the JoinSet to complete (abnormal shutdown)
                join_result = self.join_set.join_next() => {
                    if self.handle_secondary_task_completion(join_result) {
                        break;
                    }
                }
            }
        }
        info!("NetsimDaemon main loop exited.");
        RunResult::ExitedNormally
    }
}

pub async fn run() -> RunResult {
    match NetsimDaemon::new().await {
        Ok(StartUpMode::Owner(daemon, _ini_guard)) => daemon.run_daemon().await,
        Ok(StartUpMode::Client(config)) => {
            // If we are just a client, we shouldn't necessarily fail, but if the user
            // expected to start a NEW daemon, they might be confused.
            // For now, valid behavior is to print info and exit normally (acting as a
            // client/discovery).
            info!("Another netsimd is running. Will use its config: {:?}", config);
            info!("Target gRPC port: {}", config.grpc_port);
            RunResult::ExitedNormally
        }
        Err(e) => e,
    }
}
