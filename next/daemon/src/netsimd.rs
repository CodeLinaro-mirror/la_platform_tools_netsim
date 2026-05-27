// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    env, io,
    path::PathBuf,
    sync::{Arc, atomic::AtomicU32},
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
use futures::{FutureExt, SinkExt, StreamExt, pin_mut};
use grpc_server::PacketStreamerService;
use link_actor::LinkClient;
use netsim_model::{
    BluetoothMode, ChipClient, ChipInfo, ChipKind, DeviceParams, PacketSink as ApiPacketSink,
    PacketStream as ApiPacketStream, Pose, set_if_some,
};
use packet_stream::{
    StreamAddress, Streams,
    transport::traits::{PacketSink, PacketStream},
};
#[cfg(not(feature = "cuttlefish"))]
use slirp_actor::SlirpClient;
use tokio::{
    signal::{self},
    sync::mpsc,
    task::JoinSet,
};
use tracing::{error, info, warn};
#[cfg(feature = "cuttlefish")]
use {
    packet_stream::{
        ChipInfo as PsChipInfo, DualFdConfig, DualFdListener, InitInfo as PsInitInfo,
        transport::traits::TransportListener,
    },
    tokio::{net::TcpStream, select},
    tokio_util::codec::{Framed, LengthDelimitedCodec},
};

use crate::{
    args::Args,
    ini_file::{IniFile, IniFileAccess, IniFileInitialized, IniFileUninitialized, NetsimConfig},
    logger,
    version::get_version,
};

const MAX_INIT_RETRIES: i32 = 20;
const RETRY_DELAY: Duration = Duration::from_millis(100);

#[derive(Debug, PartialEq)]
pub enum RunResult {
    ExitedNormally,
    InitializationError(String),
}

fn init_error<E: std::fmt::Display>(e: E) -> RunResult {
    RunResult::InitializationError(e.to_string())
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum StartUpMode {
    Owner(NetsimDaemon, IniFileInitialized),
    Client(NetsimConfig),
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

    let chip_variant = match chip.kind {
        ChipKind::BLUETOOTH => {
            Some(netsim_model::ChipVariant::Bluetooth(Box::new(netsim_model::Bluetooth {
                address: chip.address.clone(),
                bt_properties: Default::default(),
                mode: BluetoothMode::Device(DeviceParams {}),
                ..Default::default()
            })))
        }
        ChipKind::UWB => {
            Some(netsim_model::ChipVariant::Uwb(netsim_model::Uwb { radio: Default::default() }))
        }
        ChipKind::WIFI => {
            Some(netsim_model::ChipVariant::Wifi(netsim_model::Wifi { radio: Default::default() }))
        }
        ChipKind::CELLULAR => Some(netsim_model::ChipVariant::Cell(netsim_model::Cell::default())),
        ChipKind::ETHERNET | ChipKind::CELLULAR_DATA => None,
        kind => {
            error!("Unsupported chip kind: {:?}", kind);
            return;
        }
    };

    let chip_instance = netsim_model::Chip {
        id: 0,
        kind: chip.kind,
        name: chip.name.clone(),
        manufacturer: chip.manufacturer.clone(),
        product_name: chip.product_name.clone(),
        variant: chip_variant,
        ..Default::default()
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

    let api_sink: ApiPacketSink = Box::pin(sink.sink_map_err(io::Error::other));

    let request = DeviceAddChip {
        device_guid,
        packet_stream: Some(api_stream),
        packet_sink: Some(api_sink),
        device_config,
        chip: chip_instance,
    };

    if let Err(e) = device_client.add_chip(request).await {
        error!("Failed to register stream for {}: {}", chip_info.name, e);
    }
}

#[allow(clippy::too_many_arguments)]
async fn setup_grpc_listener(
    streams: &mut Streams,
    listener_addresses: &mut HashMap<String, StreamAddress>,
    requested_port: u16,
    enable_cli_ui: bool,
    device_client: DeviceClient,
    link_client: LinkClient,
    #[cfg(not(feature = "cuttlefish"))] ap_client: ap_actor::ApClient,
    version: String,
) -> Result<(u16, grpcio::Server), RunResult> {
    // Create a channel to bridge PacketStreamerService connections to Streams
    let (new_connection_tx, new_connection_rx) = mpsc::channel(100);
    let packet_streamer_service = PacketStreamerService::new(new_connection_tx);

    // Start the gRPC server
    let (server, port) = grpc_server::start(
        requested_port.into(),
        enable_cli_ui,
        device_client,
        link_client,
        #[cfg(not(feature = "cuttlefish"))]
        ap_client,
        packet_streamer_service,
        version,
    )
    .map_err(|e| init_error(format!("Failed to start gRPC server: {}", e)))?;

    let listener = grpc_server::ChannelTransportListener {
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

    #[cfg(not(feature = "cuttlefish"))]
    slirp_client: Option<SlirpClient>,
    chip_clients: HashMap<ChipKind, Box<dyn ChipClient>>,
}

impl NetsimDaemon {
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
    #[allow(clippy::new_ret_no_self)]
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

        logger::init("netsim", args.verbose);

        #[cfg(feature = "cuttlefish")]
        if let Some(connector_instance) = args.connector_instance {
            info!("netsim startup (Connector mode)");
            return match Self::run_netsimd_connector(args, connector_instance).await {
                Ok(()) => Err(RunResult::ExitedNormally),
                Err(err) => Err(RunResult::InitializationError(err)),
            };
        }

        info!("netsim startup");

        // enable Rust backtrace by setting env RUST_BACKTRACE=full
        // SAFETY: Single-threaded initialization code. Caller must guarantee this.
        unsafe {
            env::set_var("RUST_BACKTRACE", "full");
        }

        // Log where netsim artifacts are located
        info!("Artifacts: {:?}", netsimd_temp_dir());
        // Log all args
        info!("{args:#?}");

        // Resolve TAP configuration early to validate permissions/availability.
        #[cfg(all(target_os = "linux", not(feature = "cuttlefish")))]
        let wifi_tap = args.wifi.wifi_tap.clone().or_else(|| {
            if args.wifi.wifi_cvd_tap { Some("cvd-etap-%02d".to_string()) } else { None }
        });

        // Pre-check TAP permissions if configured.
        // We do this BEFORE redirection so the user can see the error in the console.
        #[cfg(all(target_os = "linux", not(feature = "cuttlefish")))]
        if let Some(ref tap_config) = wifi_tap
            && let Err(e) = wifi_actor::TapGateway::preflight_check(tap_config)
        {
            return Err(RunResult::InitializationError(format!("TAP configuration failed: {}", e)));
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

        let mut attempts = 0;
        // Support Cuttlefish multi-instance by using instance-specific INI files.
        let instance_num = get_instance(args.instance);

        loop {
            let ini_file =
                IniFile::new_for_dir(discovery_dir.clone(), instance_num).map_err(init_error)?;

            // Attempt to acquire the singleton lock for the netsim daemon.
            // The lock is managed by direct file locking on the netsim.ini file.
            match ini_file.try_acquire().map_err(init_error)? {
                // This instance is the Writer (the primary daemon).
                IniFileAccess::Writer(uninitialized_guard) => {
                    return Self::initialize_primary_daemon(uninitialized_guard, args).await;
                }
                // This instance is a Reader, another daemon is already running.
                IniFileAccess::Reader(config) => {
                    info!("Lock held by another process. Reading config: {:?}", config);
                    return Ok(StartUpMode::Client(config));
                }
                IniFileAccess::Initializing => {
                    if attempts >= MAX_INIT_RETRIES {
                        return Err(init_error("Timed out waiting for daemon to initialize"));
                    }
                    attempts += 1;
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
        }
    }

    async fn initialize_primary_daemon(
        ini_guard: IniFileUninitialized,
        args: Args,
    ) -> Result<StartUpMode, RunResult> {
        info!("Acquired lock (Owner)");
        info!("INI file path: {}", ini_guard.path().display());

        // Initialize listeners (UDS, gRPC).
        let mut listener_addresses = HashMap::new();
        let mut streams = Streams::new();

        #[cfg(all(target_os = "linux", feature = "cuttlefish"))]
        if let Some(fd_startup_str) = &args.fd_startup_str {
            Self::init_dualfd_listener(fd_startup_str, &mut streams).await;
        }

        // Setup Link Server
        let (link_runner, link_client) = link_actor::new();

        // Setup Device Server Channel
        let (device_runner, device_client) = device_actor::new();

        // Setup Capture Server
        let (capture_runner, capture_client) = capture_actor::new();

        // Coordinate IDs across all actors
        let next_chip_id = Arc::new(AtomicU32::new(0));

        #[cfg(not(feature = "cuttlefish"))]
        let (
            ap_runner,
            ap_client,
            ap_actor_state,
            slirp_runner,
            slirp_client,
            slirp_actor_state,
            wifi_runner,
            wifi_client,
            wifi_actor_state,
            eth_runner,
            eth_client,
            eth_actor_state,
        ) = {
            let shared_keys = Arc::new(ap_actor::SharedKeyStore::new());
            let (ap_runner, ap_client) = ap_actor::new();
            let ap_actor_state = ap_actor::ApActor::new(shared_keys.clone());

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
            let wifi_tap = {
                #[cfg(target_os = "linux")]
                {
                    args.wifi.wifi_tap.clone().or_else(|| {
                        if args.wifi.wifi_cvd_tap {
                            Some("cvd-etap-%02d".to_string())
                        } else {
                            None
                        }
                    })
                }
                #[cfg(not(target_os = "linux"))]
                {
                    None
                }
            };

            // TAP preflight check is now done in `new_with_dirs` before lock acquisition.

            let wifi_actor_state = wifi_actor::WifiActor::new(
                Some(Arc::new(ap_client.clone())),
                Some(slirp_client.clone()),
                device_client.clone(),
                wifi_tap,
                shared_keys,
                Arc::new(wifi_actor::SystemClock),
                args.forward_host_mdns,
            );

            // Setup Ethernet Actor
            let (eth_runner, eth_client) = ethernet_actor::new();
            let eth_actor_state =
                ethernet_actor::EthernetActor::new(slirp_client.clone(), device_client.clone());

            (
                ap_runner,
                ap_client,
                ap_actor_state,
                slirp_runner,
                slirp_client,
                slirp_actor_state,
                wifi_runner,
                wifi_client,
                wifi_actor_state,
                eth_runner,
                eth_client,
                eth_actor_state,
            )
        };

        // gRPC port is determined after the listener starts.
        let (actual_grpc_port, grpc_server) = setup_grpc_listener(
            &mut streams,
            &mut listener_addresses,
            args.grpc_port.unwrap_or(0),
            !args.no_cli_ui,
            device_client.clone(),
            link_client.clone(),
            #[cfg(not(feature = "cuttlefish"))]
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
        let mut actual_ws_port = None;
        let websocket_port = args.ws_port.map(|p| p + instance_num - 1);
        if let Some(ws_port) = websocket_port {
            match websocket_server::server::bind(ws_port) {
                Ok(listener) => {
                    actual_ws_port = Some(listener.local_addr().map_err(init_error)?.port());
                    tokio::spawn(websocket_server::server::run(listener, device_client.clone()));
                }
                Err(e) => {
                    error!("Failed to bind WebSocket server: {e}");
                }
            }
        }

        let mut actual_tcp_port = None;
        let should_start_tcp =
            if cfg!(feature = "cuttlefish") { true } else { args.tcp_port.is_some() };

        if should_start_tcp {
            let tcp_port = args.tcp_port.unwrap_or(0);
            if let Err(e) = streams
                .start_listener(
                    "tcp",
                    packet_stream::transport::TransportType::tcp("127.0.0.1", tcp_port),
                )
                .await
            {
                error!("Failed to start TCP forwarder listener: {e}");
            } else if let Some(packet_stream::StreamAddress::Tcp(socket_addr)) =
                streams.listener_address("tcp")
            {
                actual_tcp_port = Some(socket_addr.port());
            }
        }
        // Write the current daemon's information to the INI file.
        // Clients will use this to connect.
        let mut ini_data = HashMap::from([
            ("pid".to_string(), std::process::id().to_string()),
            ("grpc.port".to_string(), actual_grpc_port.to_string()),
            ("hci.port".to_string(), hci_port.to_string()),
        ]);
        if let Some(port) = actual_tcp_port {
            ini_data.insert("tcp.port".to_string(), port.to_string());
        }
        if let Some(ws_port) = actual_ws_port {
            ini_data.insert("ws.port".to_string(), ws_port.to_string());
        }
        if let Some(StreamAddress::Uds(path)) = listener_addresses.get("netsim_uds") {
            ini_data.insert("uds.path".to_string(), path.to_string_lossy().to_string());
        }

        // Even if stale file removal failed, we can proceed as ini_guard.write will
        // overwrite.
        let initialized_guard = ini_guard.write(&ini_data).map_err(init_error)?;
        info!("Wrote to INI file {}", initialized_guard.path().display());

        // Setup Bluetooth Server
        let (bt_runner, bt_client) = bluetooth_actor::new();
        let bt_actor_state = bluetooth_actor::BluetoothActor::new(device_client.clone());

        // Setup Uwb Server
        let (uwb_runner, uwb_client) = uwb_actor::new();
        let uwb_actor = uwb_actor::UwbActor::new(device_client.clone());

        // Setup Cell Server
        let (cell_runner, cell_client) = cell_actor::new();
        let cell_actor_state = cell_actor::CellActor::new(device_client.clone());

        // Prepare chip clients map for DeviceServer
        let mut chip_clients: HashMap<ChipKind, Box<dyn ChipClient>> = HashMap::new();
        chip_clients.insert(ChipKind::BLUETOOTH, Box::new(bt_client.clone()));
        #[cfg(not(feature = "cuttlefish"))]
        chip_clients.insert(ChipKind::WIFI, Box::new(wifi_client.clone()));
        #[cfg(not(feature = "cuttlefish"))]
        chip_clients.insert(ChipKind::ETHERNET, Box::new(eth_client.clone()));
        #[cfg(not(feature = "cuttlefish"))]
        chip_clients.insert(ChipKind::CELLULAR_DATA, Box::new(eth_client.clone()));
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
        #[cfg(not(feature = "cuttlefish"))]
        join_set.spawn(wifi_runner.run(wifi_actor_state));
        #[cfg(not(feature = "cuttlefish"))]
        join_set.spawn(eth_runner.run(eth_actor_state));
        #[cfg(not(feature = "cuttlefish"))]
        join_set.spawn(ap_runner.run(ap_actor_state));
        #[cfg(not(feature = "cuttlefish"))]
        join_set.spawn(slirp_runner.run(slirp_actor_state));
        join_set.spawn(cell_runner.run(cell_actor_state));
        join_set.spawn(link_runner.run(link_actor_state));
        join_set.spawn(capture_runner.run(capture_actor::CaptureActor::new(args.pcap, None)));
        join_set.spawn(uwb_runner.run(uwb_actor));

        // Spawn DeviceActor separately
        device_actor_state.set_self_client(device_client.clone());
        let device_task = tokio::spawn(device_runner.run(device_actor_state));

        // Create Default AP
        #[cfg(not(feature = "cuttlefish"))]
        {
            let mut ap_config = ap_actor::ApConfig::default();
            if let Some(ssid) = &args.wifi.wifi_ssid {
                ap_config.ssid = ssid.clone();
            }
            set_if_some!(ap_config.wpa_passphrase, args.wifi.wifi_password.clone(), Some);
            set_if_some!(ap_config.channel, args.wifi.wifi_channel);
            set_if_some!(ap_config.beacon_interval, args.wifi.wifi_beacon_interval);
            set_if_some!(ap_config.hw_mode, args.wifi.wifi_mode, Into::into);

            ap_client.create_ap(Some(0), ap_config).await.expect("Failed to create default AP");
        }

        // Create test beacons if required
        let test_beacons = match (args.test_beacons, args.no_test_beacons) {
            (true, false) => true,
            (false, true) => false,
            (false, false) => cfg!(feature = "cuttlefish"),
            (true, true) => panic!("unexpected flag combination"),
        };

        if test_beacons {
            crate::test_beacons::create_test_beacons(&device_client).await;
        }

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
                #[cfg(not(feature = "cuttlefish"))]
                slirp_client: Some(slirp_client.clone()),
                chip_clients: daemon_chip_clients,
            },
            initialized_guard,
        ))
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
        #[cfg(not(feature = "cuttlefish"))]
        let slirp_fut = async {
            if let Some(slirp) = &self.slirp_client
                && let Err(e) = slirp.shutdown().await
            {
                warn!("SlirpActor shutdown error: {}", e);
            }
        };
        #[cfg(feature = "cuttlefish")]
        let slirp_fut = async {};
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

    #[cfg(all(target_os = "linux", feature = "cuttlefish"))]
    async fn init_dualfd_listener(fd_startup_str: &str, streams: &mut Streams) {
        if fd_startup_str.is_empty() {
            return;
        }
        match serde_json::from_str::<DualFdConfig>(fd_startup_str) {
            Ok(config) => match DualFdListener::new(config).await {
                Ok(listener) => {
                    if let Err(e) = streams.add_listener("netsim_dualfd", Box::new(listener)) {
                        error!("Failed to add DualFdListener: {}", e);
                    } else {
                        info!("Added DualFdListener");
                    }
                }
                Err(e) => {
                    error!("Failed to create DualFdListener: {}", e);
                }
            },
            Err(e) => {
                error!("Failed to parse fd_startup_str JSON: {}", e);
            }
        }
    }

    /// Runs the main event loop for the daemon.
    pub async fn run_daemon(mut self) -> RunResult {
        info!("Netsimd started {}", if self.args.no_shutdown { "--no-shutdown" } else { "" });

        let shutdown_signal = shutdown_signal();
        pin_mut!(shutdown_signal);

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
                // Branch 4: Graceful shutdown
                () = &mut shutdown_signal => {
                    info!("Shutting down gracefully...");
                    self.shutdown_actors().await;
                    break;
                }
            }
        }
        info!("NetsimDaemon main loop exited.");
        RunResult::ExitedNormally
    }
}

/// Listens for process shutdown signals to enable graceful termination.
///
/// - On Unix: listens for `SIGINT` (Ctrl+C) and `SIGTERM`.
/// - On Windows: listens for Ctrl+C and `CTRL_CLOSE_EVENT`.
///
/// If it fails to listen for a signal, that error is logged and ignored.
async fn shutdown_signal() {
    let ctrl_c = signal::ctrl_c().fuse();
    pin_mut!(ctrl_c);

    let mut terminate = {
        #[cfg(unix)]
        {
            match signal::unix::signal(signal::unix::SignalKind::terminate()) {
                Ok(sig) => Some(sig),
                Err(err) => {
                    warn!("Failed to listen for terminate signal: {err}");
                    None
                }
            }
        }
        #[cfg(target_os = "windows")]
        {
            match signal::windows::ctrl_close() {
                Ok(sig) => Some(sig),
                Err(err) => {
                    warn!("Failed to listen for ctrl_close signal: {err}");
                    None
                }
            }
        }
        #[cfg(not(any(unix, target_os = "windows")))]
        {
            None
        }
    };
    let terminate_fut = async {
        if let Some(ref mut sig) = terminate {
            sig.recv().await;
        } else {
            // Signal failed to init or unsupported platform
            futures::future::pending::<Option<()>>().await;
        }
    };
    pin_mut!(terminate_fut);

    loop {
        tokio::select! {
            res = &mut ctrl_c => {
                if let Err(err) = res {
                    warn!("Failed to listen for ctrl+c signal: {err}");
                } else {
                    info!("Ctrl+C received");
                    break;
                }
            }
            _ = &mut terminate_fut => {
                info!("Termination signal received");
                break;
            }
        }
    }
}

#[tokio::main]
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

#[cfg(feature = "cuttlefish")]
impl NetsimDaemon {
    async fn run_chip_bridge(
        mut packet_stream: PacketStream,
        mut packet_sink: PacketSink,
        chip_info: PsChipInfo,
        server_addr: String,
    ) {
        let chip_id = chip_info.chip.as_ref().map_or("Unknown".to_string(), |c| c.id.clone());

        loop {
            // Connect to TCP port
            let tcp_stream_raw = match TcpStream::connect(&server_addr).await {
                Ok(stream) => stream,
                Err(e) => {
                    warn!("Failed to connect to primary daemon TCP port, retrying in 1s: {e}");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                }
            };

            // Create InitInfo
            let init_info = PsInitInfo {
                chip_info: chip_info.clone(),
                transport_type: "TCP Forwarder".to_string(),
            };

            // Send InitInfo with LengthDelimitedCodec
            let mut framed_tcp = Framed::new(tcp_stream_raw, LengthDelimitedCodec::new());

            let init_json = match serde_json::to_vec(&init_info) {
                Ok(json) => json,
                Err(e) => {
                    error!("Failed to serialize InitInfo: {e}");
                    break;
                }
            };

            if let Err(e) = framed_tcp.send(bytes::Bytes::from(init_json)).await {
                error!("Failed to send InitInfo: {e}");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }

            info!("Sent initial ChipInfo to primary daemon for chip {chip_id}");

            let (mut tcp_sink, mut tcp_stream) = framed_tcp.split();

            enum BridgeError {
                Guest(String),
                Tcp(String),
            }

            // Poll both concurrently.
            let is_guest_disconnected = select! {
                res = async {
                    while let Some(packet) = packet_stream.next().await {
                        let packet = packet.map_err(|e| BridgeError::Guest(format!("read error: {e:?}")))?;
                        tcp_sink.send(packet).await.map_err(|e| BridgeError::Tcp(format!("write error: {e:?}")))?;
                    }
                    Ok::<(), BridgeError>(())
                } => {
                    if let Err(e) = res {
                        match e {
                            BridgeError::Guest(msg) => {
                                error!("Upstream bridge failed for chip {chip_id} (Guest): {msg}");
                                true
                            }
                            BridgeError::Tcp(msg) => {
                                error!("Upstream bridge failed for chip {chip_id} (TCP): {msg}");
                                false
                            }
                        }
                    } else {
                        true // Guest FD reader hit EOF, break loop permanently!
                    }
                }
                res = async {
                    while let Some(packet) = tcp_stream.next().await {
                        let packet = packet.map_err(|e| BridgeError::Tcp(format!("read error: {e:?}")))?;
                        packet_sink.send(packet.freeze()).await.map_err(|e| BridgeError::Guest(format!("write error: {e:?}")))?;
                    }
                    Ok::<(), BridgeError>(())
                } => {
                    if let Err(e) = res {
                        match e {
                            BridgeError::Guest(msg) => {
                                error!("Downstream bridge failed for chip {chip_id} (Guest): {msg}");
                                true
                            }
                            BridgeError::Tcp(msg) => {
                                error!("Downstream bridge failed for chip {chip_id} (TCP): {msg}");
                                false
                            }
                        }
                    } else {
                        false // TCP stream disconnected, need to reconnect
                    }
                }
            };

            if is_guest_disconnected {
                info!("Guest radio transport disconnected permanently for chip {chip_id}");
                break;
            }

            info!("TCP stream disconnected, re-establishing session in 1s for chip {chip_id}...");
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
        info!("Duplex bridge closed for chip {chip_id}");
    }

    /// Orchestrates the Cuttlefish packet forwarder connector daemon.
    async fn run_netsimd_connector(args: Args, connector_instance: u16) -> Result<(), String> {
        let Some(fd_startup_str) = args.fd_startup_str else {
            return Err("Failed to start netsimd forwarder, missing `-s` arg".to_string());
        };

        let dual_fd_config: DualFdConfig = serde_json::from_str(&fd_startup_str)
            .map_err(|e| format!("Could not parse startup info JSON: {e}"))?;

        let Some(server_addr) = common::util::ini_file::get_tcp_server_address(connector_instance)
        else {
            return Err(format!(
                "No primary netsimd tcp port found for instance {connector_instance}"
            ));
        };

        info!("Starting in Connector mode to {server_addr}");

        let mut join_set = JoinSet::new();

        let mut listener = DualFdListener::new(dual_fd_config)
            .await
            .map_err(|e| format!("Failed to create DualFdListener: {e}"))?;

        loop {
            tokio::select! {
                res = listener.accept() => {
                    match res {
                        Ok((packet_stream, packet_sink, chip_info, _guid)) => {
                            join_set.spawn(Self::run_chip_bridge(
                                packet_stream,
                                packet_sink,
                                chip_info,
                                server_addr.clone(),
                            ));
                        }
                        Err(e) => {
                            error!("Error accepting connection: {e}");
                            break;
                        }
                    }
                }
                res = join_set.join_next(), if !join_set.is_empty() => {
                    if let Some(res) = res {
                        if let Err(e) = res {
                            return Err(format!("Bridge task panicked: {e}"));
                        }
                        return Err("A guest radio transport disconnected".to_string());
                    }
                }
            }
        }

        info!("Connector forwarder disconnected unexpectedly");
        Err("Connector forwarder disconnected unexpectedly from guest FDs".to_string())
    }
}
