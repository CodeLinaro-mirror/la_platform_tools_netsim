// Copyright 2023-2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use bytes::Bytes;
use log::{debug, error, info, trace, warn};
use slirp::{
    Config, ConnectionArgs, HostFwdRule, Proto, Slirp, SlirpRequest, SlirpResponse,
    packet::{IpPacket, NetworkPacket, ParsedPacket, TransportPacket},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, split},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    task::JoinHandle,
};
use tokio_tun::Tun;

struct TcpConnection {
    writer: mpsc::Sender<Bytes>,
    reader_handle: JoinHandle<()>,
    writer_handle: JoinHandle<()>,
}

struct UdpConnection {
    writer: mpsc::Sender<Bytes>,
    reader_handle: JoinHandle<()>,
    writer_handle: JoinHandle<()>,
}

struct IcmpConnection {
    writer: mpsc::Sender<Bytes>,
    reader_handle: JoinHandle<()>,
    writer_handle: JoinHandle<()>,
}

enum Connection {
    Connecting(Option<JoinHandle<()>>),
    Tcp(TcpConnection),
    Udp(UdpConnection),
    Icmp(IcmpConnection),
}

pub struct TokioHost {
    slirp_request_sender: mpsc::Sender<SlirpRequest>,
    slirp_response_receiver: mpsc::Receiver<SlirpResponse>,
    guest_packet_sender: mpsc::Sender<Bytes>,
    timer_task: Option<JoinHandle<()>>,
    test_slirp_response_sender: Option<mpsc::Sender<SlirpResponse>>,
    fast_path_enabled: bool,
    fast_path_connections: Arc<Mutex<HashMap<SocketAddr, mpsc::Sender<Bytes>>>>,
    connections: Arc<Mutex<HashMap<u64, Connection>>>,
    hostfwd_tasks: Vec<JoinHandle<()>>,
    socks5_proxy: Option<SocketAddr>,
    slirp_task: Option<JoinHandle<()>>,
    tun_reader_task: Option<JoinHandle<()>>,
}

impl TokioHost {
    pub async fn new(tun: Tun, mut config: Config, fast_path_enabled: bool) -> Self {
        // If `dns_servers` is empty or matches the default configuration [8.8.8.8],
        // automatically discover host DNS servers so guest queries are not
        // routed to hardcoded public servers. Explicitly configured DNS server
        // lists are preserved and never overridden.
        if config.dns_servers.is_empty() || config.dns_servers == [slirp::DEFAULT_DNS_SERVER] {
            let discovered = slirp::discover_host_dns_servers().await;
            if !discovered.is_empty() {
                config.dns_servers = discovered;
            } else if config.dns_servers.is_empty() {
                config.dns_servers = vec![slirp::DEFAULT_DNS_SERVER];
            }
        }
        let socks5_proxy = config.socks5_proxy;
        let (slirp_request_sender, mut slirp_request_receiver) = mpsc::channel(10000);
        let (slirp_response_sender, slirp_response_receiver) = mpsc::channel(10000);
        let (guest_packet_sender, mut guest_packet_receiver) = mpsc::channel::<Bytes>(10000);

        let mut slirp = Slirp::new(config.clone());

        // Slirp task
        let slirp_task = tokio::spawn(async move {
            info!("slirp task started");
            while let Some(request) = slirp_request_receiver.recv().await {
                trace!("slirp task received request: {request:?}");
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    slirp.handle_request(request)
                }));

                if let Ok(responses) = result {
                    trace!("slirp task sending responses: {responses:?}");
                    for response in responses {
                        if slirp_response_sender.send(response).await.is_err() {
                            break;
                        }
                    }
                } else {
                    error!("PANIC in slirp task");
                }
            }
            info!("slirp task finished");
        });

        let (mut tun_reader, mut tun_writer) = split(tun);
        let fast_path_connections: Arc<Mutex<HashMap<SocketAddr, mpsc::Sender<Bytes>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let fast_path_connections_clone = fast_path_connections.clone();

        tokio::spawn(async move {
            while let Some(packet) = guest_packet_receiver.recv().await {
                if tun_writer.write_all(&packet).await.is_err() {
                    break;
                }
            }
        });

        let slirp_task_sender_clone = slirp_request_sender.clone();
        let tun_reader_task = tokio::spawn(async move {
            let mut buf = [0u8; 65536];
            loop {
                match tun_reader.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        let packet_buf = &buf[..n];
                        if fast_path_enabled {
                            if let Some(packet) = ParsedPacket::parse(packet_buf) {
                                if let (
                                    Some(network),
                                    Some(TransportPacket::Tcp(tcp_header, tcp_payload)),
                                ) = (packet.network, packet.transport)
                                {
                                    let src_addr = match network {
                                        NetworkPacket::Ip(IpPacket::V4(ip_header, _)) => {
                                            Some(SocketAddr::new(
                                                IpAddr::V4(Ipv4Addr::from(ip_header.source_addr)),
                                                tcp_header.source_port.get(),
                                            ))
                                        }
                                        NetworkPacket::Ip(IpPacket::V6(ip_header, _)) => {
                                            Some(SocketAddr::new(
                                                IpAddr::V6(ip_header.source_addr.into()),
                                                tcp_header.source_port.get(),
                                            ))
                                        }
                                        _ => None,
                                    };
                                    if let Some(src_addr) = src_addr {
                                        if forward_fast_path(
                                            src_addr,
                                            tcp_header.fin(),
                                            tcp_header.rst(),
                                            tcp_payload,
                                            &fast_path_connections_clone,
                                        )
                                        .await
                                        {
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                        debug!("slow path packet");
                        slirp_task_sender_clone
                            .send(SlirpRequest::Packet(Bytes::copy_from_slice(packet_buf)))
                            .await
                            .ok();
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
        });
        let connections = Arc::new(Mutex::new(HashMap::new()));
        let hostfwd_tasks =
            Self::setup_hostfwd(&config.hostfwd, slirp_request_sender.clone(), connections.clone());

        Self {
            slirp_request_sender,
            slirp_response_receiver,
            guest_packet_sender,
            timer_task: None,
            test_slirp_response_sender: None,
            fast_path_enabled,
            fast_path_connections,
            connections,
            hostfwd_tasks,
            socks5_proxy,
            slirp_task: Some(slirp_task),
            tun_reader_task: Some(tun_reader_task),
        }
    }

    fn setup_hostfwd(
        rules: &[HostFwdRule],
        slirp_request_sender: mpsc::Sender<SlirpRequest>,
        connections: Arc<Mutex<HashMap<u64, Connection>>>,
    ) -> Vec<JoinHandle<()>> {
        let next_host_conn_id = Arc::new(AtomicU64::new(1 << 60));
        let mut hostfwd_tasks = Vec::new();

        for rule in rules {
            match rule.proto {
                Proto::Tcp => {
                    let task = tokio::spawn(start_tcp_hostfwd_listener(
                        rule.clone(),
                        slirp_request_sender.clone(),
                        connections.clone(),
                        next_host_conn_id.clone(),
                    ));
                    hostfwd_tasks.push(task);
                }
                Proto::Udp => {
                    warn!("UDP hostfwd is not supported yet!");
                }
            }
        }

        hostfwd_tasks
    }

    pub fn from_channels(
        slirp_request_sender: mpsc::Sender<SlirpRequest>,
        slirp_response_receiver: mpsc::Receiver<SlirpResponse>,
        guest_packet_sender: mpsc::Sender<Bytes>,
        test_slirp_response_sender: Option<mpsc::Sender<SlirpResponse>>,
        config: Config,
    ) -> Self {
        let socks5_proxy = config.socks5_proxy;
        let connections = Arc::new(Mutex::new(HashMap::new()));
        let hostfwd_tasks =
            Self::setup_hostfwd(&config.hostfwd, slirp_request_sender.clone(), connections.clone());

        Self {
            slirp_request_sender,
            slirp_response_receiver,
            guest_packet_sender,
            timer_task: None,
            test_slirp_response_sender,
            fast_path_enabled: true, // Assume fast path for tests
            fast_path_connections: Arc::new(Mutex::new(HashMap::new())),
            connections,
            hostfwd_tasks,
            socks5_proxy,
            slirp_task: None,
            tun_reader_task: None,
        }
    }

    pub fn handle(&self) -> TokioHostHandle {
        TokioHostHandle { sender: self.slirp_request_sender.clone() }
    }

    pub async fn run(&mut self) {
        while let Some(response) = self.slirp_response_receiver.recv().await {
            trace!("host received response: {response:?}");
            if let Some(sender) = &self.test_slirp_response_sender {
                sender.send(response.clone()).await.ok();
            }
            match response {
                SlirpResponse::Packet(packet) => {
                    self.guest_packet_sender.send(packet).await.ok();
                }
                SlirpResponse::EstablishConnection(conn_id, conn_info) => match conn_info {
                    ConnectionArgs::Tcp(conn_info) => {
                        let slirp_request_sender = self.slirp_request_sender.clone();
                        let connections = self.connections.clone();
                        let socks5_proxy = self.socks5_proxy;
                        self.connections
                            .lock()
                            .unwrap()
                            .insert(conn_id, Connection::Connecting(None));
                        let handle = tokio::spawn(async move {
                            let stream =
                                match connect_tcp(conn_info.destination, socks5_proxy).await {
                                    Ok(stream) => stream,
                                    Err(e) => {
                                        warn!(
                                            "Failed to connect TCP to {}: {e}",
                                            conn_info.destination
                                        );
                                        let was_connecting = matches!(
                                            connections.lock().unwrap().remove(&conn_id),
                                            Some(Connection::Connecting(_))
                                        );
                                        if was_connecting {
                                            slirp_request_sender
                                                .send(SlirpRequest::ConnectionClosed(conn_id))
                                                .await
                                                .ok();
                                        }
                                        return;
                                    }
                                };
                            let mut conns = connections.lock().unwrap();
                            if let Some(Connection::Connecting(_)) = conns.get(&conn_id) {
                                register_tcp_connection_locked(
                                    stream,
                                    conn_id,
                                    slirp_request_sender.clone(),
                                    &mut conns,
                                );
                            } else {
                                debug!("TCP connection {conn_id} was closed while connecting");
                            }
                        });
                        let mut conns = self.connections.lock().unwrap();
                        if let Some(Connection::Connecting(h)) = conns.get_mut(&conn_id) {
                            *h = Some(handle);
                        } else {
                            handle.abort();
                        }
                    }
                    ConnectionArgs::Udp(conn_info) => {
                        if let Err(e) = register_udp_connection(
                            conn_id,
                            conn_info,
                            self.slirp_request_sender.clone(),
                            self.connections.clone(),
                        ) {
                            warn!("Failed to register UDP connection {conn_id}: {e}");
                            self.slirp_request_sender
                                .send(SlirpRequest::ConnectionClosed(conn_id))
                                .await
                                .ok();
                        }
                    }
                    ConnectionArgs::Icmp(conn_info) => {
                        if let Err(e) = register_icmp_connection(
                            conn_id,
                            conn_info,
                            self.slirp_request_sender.clone(),
                            self.connections.clone(),
                        ) {
                            warn!("Failed to register ICMP connection {conn_id}: {e}");
                            self.slirp_request_sender
                                .send(SlirpRequest::ConnectionClosed(conn_id))
                                .await
                                .ok();
                        }
                    }
                },
                SlirpResponse::WriteToConnection(conn_id, data) => {
                    let sender = {
                        let conns = self.connections.lock().unwrap();
                        match conns.get(&conn_id) {
                            Some(Connection::Tcp(c)) => Some(c.writer.clone()),
                            Some(Connection::Udp(c)) => Some(c.writer.clone()),
                            Some(Connection::Icmp(c)) => Some(c.writer.clone()),
                            Some(Connection::Connecting(_)) | None => None,
                        }
                    };
                    if let Some(sender) = sender {
                        sender.send(data).await.ok();
                    }
                }
                SlirpResponse::ActivateFastPath { conn_id, guest_addr, .. } => {
                    trace!("activating fast path for {guest_addr}");
                    if self.fast_path_enabled {
                        if let Some(Connection::Tcp(tcp_conn)) =
                            self.connections.lock().unwrap().get(&conn_id)
                        {
                            self.fast_path_connections
                                .lock()
                                .unwrap()
                                .insert(guest_addr, tcp_conn.writer.clone());
                        }
                    }
                }
                SlirpResponse::CloseConnection { conn_id, guest_addr } => {
                    info!("Closing connection {conn_id}");
                    let removed = self.connections.lock().unwrap().remove(&conn_id);
                    if let Some(conn) = removed {
                        match conn {
                            Connection::Connecting(Some(handle)) => {
                                info!("Aborting connecting task for connection {conn_id}");
                                handle.abort();
                            }
                            Connection::Connecting(None) => {
                                info!(
                                    "Connecting connection {conn_id} closed before task handle registered"
                                );
                            }
                            Connection::Tcp(conn) => {
                                info!("Aborting reader task for TCP connection {conn_id}");
                                conn.reader_handle.abort();
                                info!("Aborting writer task for TCP connection {conn_id}");
                                conn.writer_handle.abort();
                                info!("TCP connection {conn_id} tasks aborted");
                            }
                            Connection::Icmp(conn) => {
                                info!("Aborting reader task for ICMP connection {conn_id}");
                                conn.reader_handle.abort();
                                info!("Aborting writer task for ICMP connection {conn_id}");
                                conn.writer_handle.abort();
                                info!("ICMP connection {conn_id} tasks aborted");
                            }
                            Connection::Udp(conn) => {
                                info!("Aborting reader task for UDP connection {conn_id}");
                                conn.reader_handle.abort();
                                info!("Aborting writer task for UDP connection {conn_id}");
                                conn.writer_handle.abort();
                                info!("UDP connection {conn_id} tasks aborted");
                            }
                        }
                    }
                    if self.fast_path_enabled {
                        trace!("deactivating fast path for {guest_addr}");
                        self.fast_path_connections.lock().unwrap().remove(&guest_addr);
                    }
                    info!("Connection {conn_id} closed");
                }
                SlirpResponse::SetTimer(duration) => {
                    if let Some(timer_task) = self.timer_task.take() {
                        timer_task.abort();
                    }
                    let sender = self.slirp_request_sender.clone();
                    self.timer_task = Some(tokio::spawn(async move {
                        tokio::time::sleep(duration).await;
                        sender.send(SlirpRequest::Timer).await.ok();
                    }));
                }
                SlirpResponse::Reset => {
                    info!(
                        "Received Reset response from Slirp core. Tearing down all active connections."
                    );
                    let mut conns = self.connections.lock().unwrap();
                    for (conn_id, conn) in conns.drain() {
                        info!("Aborting active connection {conn_id} due to Reset");
                        match conn {
                            Connection::Connecting(Some(handle)) => {
                                handle.abort();
                            }
                            Connection::Connecting(None) => {}
                            Connection::Tcp(tcp_conn) => {
                                tcp_conn.reader_handle.abort();
                                tcp_conn.writer_handle.abort();
                            }
                            Connection::Udp(udp_conn) => {
                                udp_conn.reader_handle.abort();
                                udp_conn.writer_handle.abort();
                            }
                            Connection::Icmp(icmp_conn) => {
                                icmp_conn.reader_handle.abort();
                                icmp_conn.writer_handle.abort();
                            }
                        }
                    }
                    self.fast_path_connections.lock().unwrap().clear();
                }
                _ => {}
            }
        }
    }

    pub fn connection_count(&self) -> usize {
        self.connections.lock().unwrap().len()
    }
}

#[derive(Clone)]
pub struct TokioHostHandle {
    sender: mpsc::Sender<SlirpRequest>,
}

impl TokioHostHandle {
    pub async fn save_state(&self) -> Result<Vec<u8>, &'static str> {
        // SlirpRequest::SaveState expects std::sync::mpsc::Sender because the core
        // slirp protocol engine is synchronous and decoupled from Tokio. Therefore,
        // we use std::sync::mpsc coupled with spawn_blocking here.
        let (tx, rx) = std::sync::mpsc::channel();
        self.sender
            .send(SlirpRequest::SaveState(tx))
            .await
            .map_err(|_| "Failed to send SaveState request to Slirp task")?;

        let state_bytes = tokio::task::spawn_blocking(move || {
            rx.recv().map_err(|_| "Failed to receive state from Slirp task")
        })
        .await
        .map_err(|_| "Spawn blocking failed")??;

        Ok(state_bytes)
    }

    pub async fn restore_state(&self, state_bytes: &[u8]) -> Result<(), &'static str> {
        self.sender
            .send(SlirpRequest::RestoreState(Bytes::copy_from_slice(state_bytes)))
            .await
            .map_err(|_| "Failed to send RestoreState request to Slirp task")?;

        Ok(())
    }
}

impl Drop for TokioHost {
    fn drop(&mut self) {
        info!("Dropping TokioHost. Cleaning up all background tasks.");
        if let Some(task) = self.slirp_task.take() {
            task.abort();
        }
        if let Some(task) = self.tun_reader_task.take() {
            task.abort();
        }
        if let Some(timer_task) = self.timer_task.take() {
            timer_task.abort();
        }
        for task in self.hostfwd_tasks.drain(..) {
            task.abort();
        }
        let mut conns = self.connections.lock().unwrap();
        for (conn_id, conn) in conns.drain() {
            match conn {
                Connection::Connecting(Some(handle)) => {
                    info!("Aborting connecting connection {conn_id} on drop");
                    handle.abort();
                }
                Connection::Connecting(None) => {}
                Connection::Tcp(tcp_conn) => {
                    info!("Aborting TCP connection {conn_id} on drop");
                    tcp_conn.reader_handle.abort();
                    tcp_conn.writer_handle.abort();
                }
                Connection::Udp(udp_conn) => {
                    info!("Aborting UDP connection {conn_id} on drop");
                    udp_conn.reader_handle.abort();
                    udp_conn.writer_handle.abort();
                }
                Connection::Icmp(icmp_conn) => {
                    info!("Aborting ICMP connection {conn_id} on drop");
                    icmp_conn.reader_handle.abort();
                    icmp_conn.writer_handle.abort();
                }
            }
        }
    }
}

async fn start_tcp_hostfwd_listener(
    rule: HostFwdRule,
    slirp_request_sender: mpsc::Sender<SlirpRequest>,
    connections: Arc<Mutex<HashMap<u64, Connection>>>,
    next_host_conn_id: Arc<AtomicU64>,
) {
    let listener = match TcpListener::bind(rule.host_addr).await {
        Ok(l) => l,
        Err(e) => {
            error!("Failed to bind TCP hostfwd listener on {}: {}", rule.host_addr, e);
            return;
        }
    };
    info!("TCP hostfwd listener listening on {}", rule.host_addr);

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let conn_id = next_host_conn_id.fetch_add(1, Ordering::SeqCst);
                let slirp_request_sender = slirp_request_sender.clone();
                let task_connections = connections.clone();
                let guest_addr = rule.guest_addr;

                connections.lock().unwrap().insert(conn_id, Connection::Connecting(None));
                let handle = tokio::spawn(async move {
                    // Notify Slirp core to accept the incoming connection and start handshake
                    slirp_request_sender
                        .send(SlirpRequest::AcceptIncoming {
                            conn_id,
                            host_addr: rule.host_addr,
                            guest_addr,
                        })
                        .await
                        .ok();

                    let mut conns = task_connections.lock().unwrap();
                    if let Some(Connection::Connecting(_)) = conns.get(&conn_id) {
                        register_tcp_connection_locked(
                            stream,
                            conn_id,
                            slirp_request_sender.clone(),
                            &mut conns,
                        );
                    } else {
                        debug!("TCP connection {conn_id} was closed while connecting");
                    }
                });
                let mut conns = connections.lock().unwrap();
                if let Some(Connection::Connecting(h)) = conns.get_mut(&conn_id) {
                    *h = Some(handle);
                } else {
                    handle.abort();
                }
            }
            Err(e) => {
                error!("TCP hostfwd accept error on {}: {}", rule.host_addr, e);
            }
        }
    }
}

async fn forward_fast_path(
    src_addr: SocketAddr,
    fin: bool,
    rst: bool,
    tcp_payload: &[u8],
    fast_path_connections: &Mutex<HashMap<SocketAddr, mpsc::Sender<Bytes>>>,
) -> bool {
    let sender = {
        let conns = fast_path_connections.lock().unwrap();
        conns.get(&src_addr).cloned()
    };
    if let Some(sender) = sender {
        trace!("fast path packet from {src_addr}");
        if !fin && !rst && !tcp_payload.is_empty() {
            sender.send(Bytes::copy_from_slice(tcp_payload)).await.ok();
            return true;
        }
    }
    false
}

#[allow(dead_code)]
fn register_tcp_connection(
    stream: TcpStream,
    conn_id: u64,
    slirp_request_sender: mpsc::Sender<SlirpRequest>,
    connections: Arc<Mutex<HashMap<u64, Connection>>>,
) -> mpsc::Sender<Bytes> {
    let mut conns = connections.lock().unwrap();
    register_tcp_connection_locked(stream, conn_id, slirp_request_sender, &mut conns)
}

fn register_tcp_connection_locked(
    stream: TcpStream,
    conn_id: u64,
    slirp_request_sender: mpsc::Sender<SlirpRequest>,
    conns: &mut HashMap<u64, Connection>,
) -> mpsc::Sender<Bytes> {
    let (mut reader, mut writer) = stream.into_split();
    let (tx, mut rx) = mpsc::channel::<Bytes>(1000);

    // Reader task
    let reader_slirp_request_sender = slirp_request_sender.clone();
    let reader_handle = tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) => {
                    reader_slirp_request_sender
                        .send(SlirpRequest::RemoteClosed(conn_id))
                        .await
                        .ok();
                    break;
                }
                Ok(n) => {
                    reader_slirp_request_sender
                        .send(SlirpRequest::Data(conn_id, Bytes::copy_from_slice(&buf[..n])))
                        .await
                        .ok();
                }
                Err(_) => {
                    reader_slirp_request_sender
                        .send(SlirpRequest::ConnectionClosed(conn_id))
                        .await
                        .ok();
                    break;
                }
            }
        }
    });

    // Writer task
    let writer_handle = tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            if writer.write_all(&data).await.is_err() {
                break;
            }
        }
    });

    conns.insert(
        conn_id,
        Connection::Tcp(TcpConnection { writer: tx.clone(), reader_handle, writer_handle }),
    );

    tx
}

async fn connect_tcp(
    destination: SocketAddr,
    socks5_proxy: Option<SocketAddr>,
) -> std::io::Result<TcpStream> {
    if let Some(proxy_addr) = socks5_proxy {
        debug!("Connecting to SOCKS5 proxy at {proxy_addr} to reach {destination}");
        let mut stream = TcpStream::connect(proxy_addr).await?;
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            perform_socks5_handshake(&mut stream, destination),
        )
        .await
        .map_err(|_| std::io::Error::other("SOCKS5 handshake timed out"))??;
        Ok(stream)
    } else {
        debug!("Connecting directly to {destination}");
        TcpStream::connect(destination).await
    }
}

async fn perform_socks5_handshake(
    stream: &mut TcpStream,
    destination: SocketAddr,
) -> std::io::Result<()> {
    // 1. Send greeting: VER=5, NMETHODS=1, METHODS=[0] (No Auth)
    stream.write_all(&[5, 1, 0]).await?;

    // 2. Read greeting response
    let mut greeting_resp = [0u8; 2];
    stream.read_exact(&mut greeting_resp).await?;
    if greeting_resp[0] != 5 || greeting_resp[1] != 0 {
        return Err(std::io::Error::other("SOCKS5 authentication negotiation failed"));
    }

    // 3. Send connection request: VER=5, CMD=1 (CONNECT), RSV=0
    let mut req = Vec::new();
    req.extend_from_slice(&[5, 1, 0]);

    match destination.ip() {
        IpAddr::V4(ip) => {
            req.push(1); // ATYP = IPv4
            req.extend_from_slice(&ip.octets());
        }
        IpAddr::V6(ip) => {
            req.push(4); // ATYP = IPv6
            req.extend_from_slice(&ip.octets());
        }
    }
    req.extend_from_slice(&destination.port().to_be_bytes());
    stream.write_all(&req).await?;

    // 4. Read response
    let mut resp_header = [0u8; 4];
    stream.read_exact(&mut resp_header).await?;
    if resp_header[0] != 5 || resp_header[1] != 0 {
        return Err(std::io::Error::other(format!(
            "SOCKS5 connection failed with error code: {}",
            resp_header[1]
        )));
    }

    // Skip bound address and port in the response
    let atyp = resp_header[3];
    match atyp {
        1 => {
            let mut buf = [0u8; 6];
            stream.read_exact(&mut buf).await?;
        }
        4 => {
            let mut buf = [0u8; 18];
            stream.read_exact(&mut buf).await?;
        }
        3 => {
            let mut len_buf = [0u8; 1];
            stream.read_exact(&mut len_buf).await?;
            let len = len_buf[0] as usize;
            let mut buf = vec![0u8; len + 2];
            stream.read_exact(&mut buf).await?;
        }
        _ => {
            return Err(std::io::Error::other("Unknown SOCKS5 address type in response"));
        }
    }

    Ok(())
}

fn register_udp_connection(
    conn_id: u64,
    conn_info: slirp::UdpConnectionArgs,
    slirp_request_sender: mpsc::Sender<SlirpRequest>,
    connections: Arc<Mutex<HashMap<u64, Connection>>>,
) -> std::io::Result<mpsc::Sender<Bytes>> {
    let bind_addr = match conn_info.destination {
        SocketAddr::V4(_) => "0.0.0.0:0".parse::<SocketAddr>().unwrap(),
        SocketAddr::V6(_) => "[::]:0".parse::<SocketAddr>().unwrap(),
    };
    let std_socket = std::net::UdpSocket::bind(bind_addr)?;
    std_socket.connect(conn_info.destination)?;
    std_socket.set_nonblocking(true)?;
    let tokio_socket = tokio::net::UdpSocket::from_std(std_socket)?;
    let socket = Arc::new(tokio_socket);

    let (tx, mut rx) = mpsc::channel::<Bytes>(100);

    // Writer task
    let writer_socket = socket.clone();
    let writer_handle = tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            writer_socket.send(&data).await.ok();
        }
    });

    // Reader task
    let reader_socket = socket.clone();
    let slirp_sender = slirp_request_sender.clone();
    let reader_handle = tokio::spawn(async move {
        let mut buf = [0u8; 65536];
        loop {
            match reader_socket.recv(&mut buf).await {
                Ok(n) => {
                    slirp_sender
                        .send(SlirpRequest::Data(conn_id, Bytes::copy_from_slice(&buf[..n])))
                        .await
                        .ok();
                }
                Err(_) => {
                    slirp_sender.send(SlirpRequest::ConnectionClosed(conn_id)).await.ok();
                    break;
                }
            }
        }
    });

    connections.lock().unwrap().insert(
        conn_id,
        Connection::Udp(UdpConnection { writer: tx.clone(), reader_handle, writer_handle }),
    );

    Ok(tx)
}

fn register_icmp_connection(
    conn_id: u64,
    conn_info: slirp::IcmpConnectionArgs,
    slirp_request_sender: mpsc::Sender<SlirpRequest>,
    connections: Arc<Mutex<HashMap<u64, Connection>>>,
) -> std::io::Result<mpsc::Sender<Bytes>> {
    use std::net::{IpAddr, SocketAddr};

    use socket2::{Domain, Protocol, Socket, Type};

    // 1. Open ICMP socket
    let (domain, bind_addr) = match conn_info.destination {
        IpAddr::V4(_) => (Domain::IPV4, "0.0.0.0:0".parse::<SocketAddr>().unwrap()),
        IpAddr::V6(_) => (Domain::IPV6, "[::]:0".parse::<SocketAddr>().unwrap()),
    };
    let proto = match conn_info.destination {
        IpAddr::V4(_) => Protocol::from(1),  // IPPROTO_ICMP
        IpAddr::V6(_) => Protocol::from(58), // IPPROTO_ICMPV6
    };

    let socket = Socket::new(domain, Type::DGRAM, Some(proto))?;
    socket.set_nonblocking(true)?;
    socket.bind(&bind_addr.into())?;

    let std_socket: std::net::UdpSocket = socket.into();
    let tokio_socket = tokio::net::UdpSocket::from_std(std_socket)?;
    let socket = Arc::new(tokio_socket);

    // Get the kernel-allocated ID (local port)
    let local_addr = socket.local_addr()?;
    let kernel_id = local_addr.port();
    debug!("Opened ICMP socket with kernel_id: {kernel_id} for guest_id: {}", conn_info.guest_id);

    let (tx, mut rx) = mpsc::channel::<Bytes>(100);

    // 2. Spawn writer task
    let writer_socket = socket.clone();
    let dest_ip = conn_info.destination;
    let writer_handle = tokio::spawn(async move {
        let dest_addr = SocketAddr::new(dest_ip, 0); // Port is 0 for raw IP destinations in sendto
        while let Some(data) = rx.recv().await {
            writer_socket.send_to(&data, dest_addr).await.ok();
        }
    });

    // 3. Spawn reader task
    let reader_socket = socket.clone();
    let slirp_sender = slirp_request_sender.clone();
    let guest_id = conn_info.guest_id;
    let guest_ip = conn_info.guest_ip;
    let is_ipv6 = conn_info.destination.is_ipv6();
    let reader_handle = tokio::spawn(async move {
        let mut buf = [0u8; 65536];
        loop {
            match reader_socket.recv_from(&mut buf).await {
                Ok((n, src_addr)) => {
                    let mut packet_data = buf[..n].to_vec();
                    // Rewrite the ICMP ID back to the guest's original ID only for Echo
                    // Reply/Request!
                    if !is_ipv6
                        && packet_data.len() >= 8
                        && (packet_data[0] == 0 || packet_data[0] == 8)
                    {
                        packet_data[4..6].copy_from_slice(&guest_id.to_be_bytes());
                        // Recalculate IPv4 ICMP checksum
                        packet_data[2..4].copy_from_slice(&[0, 0]);
                        let checksum = netsim_packets::ipv4_checksum(&packet_data);
                        packet_data[2..4].copy_from_slice(&checksum.to_be_bytes());
                    } else if is_ipv6
                        && packet_data.len() >= 8
                        && (packet_data[0] == 128 || packet_data[0] == 129)
                    {
                        packet_data[4..6].copy_from_slice(&guest_id.to_be_bytes());
                        // Recalculate ICMPv6 checksum
                        packet_data[2..4].copy_from_slice(&[0, 0]);
                        if let (IpAddr::V6(src_v6), IpAddr::V6(dst_v6)) = (src_addr.ip(), guest_ip)
                        {
                            let checksum =
                                netsim_packets::icmpv6_checksum(&packet_data, src_v6, dst_v6);
                            packet_data[2..4].copy_from_slice(&checksum.to_be_bytes());
                        }
                    }

                    slirp_sender
                        .send(SlirpRequest::Data(conn_id, Bytes::from(packet_data)))
                        .await
                        .ok();
                }
                Err(_) => {
                    slirp_sender.send(SlirpRequest::ConnectionClosed(conn_id)).await.ok();
                    break;
                }
            }
        }
    });

    connections.lock().unwrap().insert(
        conn_id,
        Connection::Icmp(IcmpConnection { writer: tx.clone(), reader_handle, writer_handle }),
    );

    Ok(tx)
}
