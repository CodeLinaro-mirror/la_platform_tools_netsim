// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#[cfg(not(target_os = "windows"))]
use std::net::IpAddr;
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};

use bytes::Bytes;
#[cfg(not(target_os = "windows"))]
use etherparse::{Icmpv4Header, Icmpv4Type, Icmpv6Header, Icmpv6Type};
#[cfg(not(target_os = "windows"))]
use socket2::{Domain, Protocol, Socket, Type};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, UdpSocket},
    sync::mpsc,
};
use tracing::{info, warn};

pub(crate) async fn run_native_slirp_loop(
    config: slirp::Config,
    downlink_tx: mpsc::UnboundedSender<Bytes>,
    mut uplink_rx: mpsc::UnboundedReceiver<Bytes>,
) {
    info!("Starting Rust native slirp engine loop");
    let mut slirp_engine = slirp::Slirp::new(config);
    let (internal_req_tx, mut internal_req_rx) = mpsc::unbounded_channel::<slirp::SlirpRequest>();
    let conn_writers: Arc<Mutex<HashMap<u64, mpsc::UnboundedSender<Bytes>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let next_timer = tokio::time::sleep(Duration::from_secs(3600 * 24 * 365));
    tokio::pin!(next_timer);

    loop {
        let req = tokio::select! {
            opt = uplink_rx.recv() => {
                match opt {
                    Some(msg) => slirp::SlirpRequest::Packet(msg),
                    None => break,
                }
            }
            opt = internal_req_rx.recv() => {
                match opt {
                    Some(req) => req,
                    None => break,
                }
            }
            _ = &mut next_timer => {
                next_timer.as_mut().reset(tokio::time::Instant::now() + Duration::from_secs(3600 * 24 * 365));
                slirp::SlirpRequest::Timer
            }
        };

        let responses = slirp_engine.handle_request(req);
        for resp in responses {
            match resp {
                slirp::SlirpResponse::Packet(pkt) => {
                    if downlink_tx.send(pkt).is_err() {
                        return;
                    }
                }
                slirp::SlirpResponse::EstablishConnection(conn_id, args) => {
                    handle_native_establish_connection(
                        conn_id,
                        args,
                        internal_req_tx.clone(),
                        conn_writers.clone(),
                    );
                }
                slirp::SlirpResponse::WriteToConnection(conn_id, data) => {
                    if let Some(tx) = conn_writers.lock().unwrap().get(&conn_id) {
                        let _ = tx.send(data);
                    }
                }
                slirp::SlirpResponse::CloseConnection { conn_id, .. } => {
                    conn_writers.lock().unwrap().remove(&conn_id);
                }
                slirp::SlirpResponse::SetTimer(dur) => {
                    next_timer.as_mut().reset(tokio::time::Instant::now() + dur);
                }
                _ => {}
            }
        }
    }
}

async fn handle_tcp_connection(
    conn_id: u64,
    dest: SocketAddr,
    mut writer_rx: mpsc::UnboundedReceiver<Bytes>,
    req_tx: mpsc::UnboundedSender<slirp::SlirpRequest>,
) {
    let stream = match TcpStream::connect(dest).await {
        Ok(s) => s,
        Err(e) => {
            info!("Failed to connect TCP to {dest}: {e}");
            let _ = req_tx.send(slirp::SlirpRequest::ConnectionClosed(conn_id));
            return;
        }
    };

    let (mut reader, mut writer) = stream.into_split();
    let req_tx_clone = req_tx.clone();
    let read_task = tokio::spawn(async move {
        let mut buf = [0u8; 8192];
        loop {
            match reader.read(&mut buf).await {
                Ok(0) => {
                    let _ = req_tx_clone.send(slirp::SlirpRequest::RemoteClosed(conn_id));
                    break;
                }
                Ok(n) => {
                    let _ = req_tx_clone.send(slirp::SlirpRequest::Data(
                        conn_id,
                        Bytes::copy_from_slice(&buf[..n]),
                    ));
                }
                Err(_) => {
                    let _ = req_tx_clone.send(slirp::SlirpRequest::ConnectionClosed(conn_id));
                    break;
                }
            }
        }
    });

    while let Some(data) = writer_rx.recv().await {
        if writer.write_all(&data).await.is_err() {
            let _ = req_tx.send(slirp::SlirpRequest::ConnectionClosed(conn_id));
            break;
        }
    }
    read_task.abort();
}

async fn handle_udp_connection(
    conn_id: u64,
    dest: SocketAddr,
    mut writer_rx: mpsc::UnboundedReceiver<Bytes>,
    req_tx: mpsc::UnboundedSender<slirp::SlirpRequest>,
) {
    let bind_addr = match dest {
        SocketAddr::V4(_) => "0.0.0.0:0",
        SocketAddr::V6(_) => "[::]:0",
    };
    let socket = match UdpSocket::bind(bind_addr).await {
        Ok(s) => {
            if s.connect(dest).await.is_err() {
                let _ = req_tx.send(slirp::SlirpRequest::ConnectionClosed(conn_id));
                return;
            }
            Arc::new(s)
        }
        Err(_) => {
            let _ = req_tx.send(slirp::SlirpRequest::ConnectionClosed(conn_id));
            return;
        }
    };

    let socket_read = socket.clone();
    let req_tx_clone = req_tx.clone();
    let read_task = tokio::spawn(async move {
        let mut buf = vec![0u8; 65535];
        loop {
            match socket_read.recv(&mut buf).await {
                Ok(n) => {
                    let _ = req_tx_clone.send(slirp::SlirpRequest::Data(
                        conn_id,
                        Bytes::copy_from_slice(&buf[..n]),
                    ));
                }
                Err(e) => {
                    warn!("UDP recv error on conn {conn_id}: {e}");
                    let _ = req_tx_clone.send(slirp::SlirpRequest::ConnectionClosed(conn_id));
                    break;
                }
            }
        }
    });

    while let Some(data) = writer_rx.recv().await {
        if socket.send(&data).await.is_err() {
            let _ = req_tx.send(slirp::SlirpRequest::ConnectionClosed(conn_id));
            break;
        }
    }
    read_task.abort();
}

#[cfg(not(target_os = "windows"))]
fn rewrite_icmp_echo_id(
    buf: &mut [u8],
    guest_id: u16,
    dest_ip: std::net::IpAddr,
    guest_ip: std::net::IpAddr,
) {
    if !dest_ip.is_ipv6() {
        if let Ok((mut header, payload)) = Icmpv4Header::from_slice(buf) {
            match header.icmp_type {
                Icmpv4Type::EchoReply(ref mut echo) | Icmpv4Type::EchoRequest(ref mut echo) => {
                    echo.id = guest_id;
                    header.update_checksum(payload);
                    buf[..header.header_len()].copy_from_slice(&header.to_bytes());
                }
                _ => {}
            }
        }
    } else if let Ok((mut header, payload)) = Icmpv6Header::from_slice(buf) {
        match header.icmp_type {
            Icmpv6Type::EchoReply(ref mut echo) | Icmpv6Type::EchoRequest(ref mut echo) => {
                echo.id = guest_id;
                if let (std::net::IpAddr::V6(src_v6), std::net::IpAddr::V6(dst_v6)) =
                    (dest_ip, guest_ip)
                {
                    let _ = header.update_checksum(src_v6.octets(), dst_v6.octets(), payload);
                    buf[..header.header_len()].copy_from_slice(&header.to_bytes());
                }
            }
            _ => {}
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn create_icmp_socket(dest_ip: IpAddr) -> std::io::Result<UdpSocket> {
    let (domain, bind_addr, proto) = match dest_ip {
        IpAddr::V4(_) => (Domain::IPV4, SocketAddr::from(([0, 0, 0, 0], 0)), Protocol::ICMPV4),
        IpAddr::V6(_) => (Domain::IPV6, SocketAddr::from(([0; 16], 0)), Protocol::ICMPV6),
    };
    let socket = Socket::new(domain, Type::DGRAM, Some(proto))?;
    socket.set_nonblocking(true)?;
    socket.bind(&bind_addr.into())?;
    UdpSocket::from_std(socket.into())
}

#[cfg(not(target_os = "windows"))]
async fn handle_icmp_connection(
    conn_id: u64,
    args: slirp::IcmpConnectionArgs,
    mut writer_rx: mpsc::UnboundedReceiver<Bytes>,
    req_tx: mpsc::UnboundedSender<slirp::SlirpRequest>,
) {
    let dest_ip = args.destination;
    let socket = match create_icmp_socket(dest_ip) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            warn!("Failed to open ICMP socket for {dest_ip}: {e}");
            let _ = req_tx.send(slirp::SlirpRequest::ConnectionClosed(conn_id));
            return;
        }
    };

    let socket_read = socket.clone();
    let req_tx_clone = req_tx.clone();
    let guest_id = args.guest_id;
    let guest_ip = args.guest_ip;

    let read_task = tokio::spawn(async move {
        let mut buf = vec![0u8; 65536];
        loop {
            match socket_read.recv_from(&mut buf).await {
                Ok((n, _)) => {
                    rewrite_icmp_echo_id(&mut buf[..n], guest_id, dest_ip, guest_ip);
                    let _ = req_tx_clone.send(slirp::SlirpRequest::Data(
                        conn_id,
                        Bytes::copy_from_slice(&buf[..n]),
                    ));
                }
                Err(_) => {
                    let _ = req_tx_clone.send(slirp::SlirpRequest::ConnectionClosed(conn_id));
                    break;
                }
            }
        }
    });

    let dest_addr = SocketAddr::new(dest_ip, 0);
    while let Some(data) = writer_rx.recv().await {
        if socket.send_to(&data, dest_addr).await.is_err() {
            let _ = req_tx.send(slirp::SlirpRequest::ConnectionClosed(conn_id));
            break;
        }
    }
    read_task.abort();
}

fn handle_native_establish_connection(
    conn_id: u64,
    args: slirp::ConnectionArgs,
    req_tx: mpsc::UnboundedSender<slirp::SlirpRequest>,
    conn_writers: Arc<Mutex<HashMap<u64, mpsc::UnboundedSender<Bytes>>>>,
) {
    let (writer_tx, writer_rx) = mpsc::unbounded_channel::<Bytes>();
    conn_writers.lock().unwrap().insert(conn_id, writer_tx);

    match args {
        slirp::ConnectionArgs::Tcp(tcp_args) => {
            tokio::spawn(handle_tcp_connection(conn_id, tcp_args.destination, writer_rx, req_tx));
        }
        slirp::ConnectionArgs::Udp(udp_args) => {
            tokio::spawn(handle_udp_connection(conn_id, udp_args.destination, writer_rx, req_tx));
        }
        slirp::ConnectionArgs::Icmp(icmp_args) => {
            #[cfg(not(target_os = "windows"))]
            tokio::spawn(handle_icmp_connection(conn_id, icmp_args, writer_rx, req_tx));
            #[cfg(target_os = "windows")]
            {
                let _ = (icmp_args, writer_rx);
                // TODO(b/554101446): Implement unprivileged Windows ICMP support (e.g. using
                // Win32 IP Helper IcmpSendEcho / Icmp6SendEcho2 from iphlpapi) when fully
                // migrating Windows Netsim to pure Rust SLIRP.
                warn!("Native SLIRP ICMP is unsupported on Windows; use C FFI backend instead");
                let _ = req_tx.send(slirp::SlirpRequest::ConnectionClosed(conn_id));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[cfg(not(target_os = "windows"))]
    use std::net::Ipv6Addr;
    use std::net::{IpAddr, Ipv4Addr};

    #[cfg(not(target_os = "windows"))]
    use etherparse::{Icmpv4Header, Icmpv4Type, Icmpv6Header, Icmpv6Type};
    use tokio::net::TcpListener;

    use super::*;

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn test_native_icmp_ipv4_handling() {
        let (req_tx, _req_rx) = mpsc::unbounded_channel::<slirp::SlirpRequest>();
        let conn_writers = Arc::new(Mutex::new(HashMap::new()));
        let conn_id = 100;
        let args = slirp::IcmpConnectionArgs {
            destination: IpAddr::V4(Ipv4Addr::LOCALHOST),
            guest_ip: IpAddr::V4(Ipv4Addr::new(10, 0, 2, 15)),
            guest_id: 0x1234,
        };

        handle_native_establish_connection(
            conn_id,
            slirp::ConnectionArgs::Icmp(args),
            req_tx,
            conn_writers.clone(),
        );

        tokio::time::sleep(Duration::from_millis(50)).await;
        let writer_tx = {
            let guard = conn_writers.lock().unwrap();
            guard.get(&conn_id).cloned()
        };
        assert!(writer_tx.is_some());
        let writer_tx = writer_tx.unwrap();

        let icmp_payload = vec![8u8, 0, 0, 0, 0x12, 0x34, 0x00, 0x01, 1, 2, 3, 4];
        let _ = writer_tx.send(Bytes::from(icmp_payload));

        conn_writers.lock().unwrap().remove(&conn_id);
    }

    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn test_native_icmp_ipv6_handling() {
        let (req_tx, _req_rx) = mpsc::unbounded_channel::<slirp::SlirpRequest>();
        let conn_writers = Arc::new(Mutex::new(HashMap::new()));
        let conn_id = 200;
        let args = slirp::IcmpConnectionArgs {
            destination: IpAddr::V6(Ipv6Addr::LOCALHOST),
            guest_ip: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
            guest_id: 0x5678,
        };

        handle_native_establish_connection(
            conn_id,
            slirp::ConnectionArgs::Icmp(args),
            req_tx,
            conn_writers.clone(),
        );

        tokio::time::sleep(Duration::from_millis(50)).await;
        let writer_tx = {
            let guard = conn_writers.lock().unwrap();
            guard.get(&conn_id).cloned()
        };
        assert!(writer_tx.is_some());
        let writer_tx = writer_tx.unwrap();

        let icmp_payload = vec![128u8, 0, 0, 0, 0x56, 0x78, 0x00, 0x01, 1, 2, 3, 4];
        let _ = writer_tx.send(Bytes::from(icmp_payload));

        conn_writers.lock().unwrap().remove(&conn_id);
    }

    #[tokio::test]
    async fn test_native_udp_handling() {
        let listener = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let local_addr = listener.local_addr().unwrap();

        let (req_tx, mut req_rx) = mpsc::unbounded_channel::<slirp::SlirpRequest>();
        let conn_writers = Arc::new(Mutex::new(HashMap::new()));
        let conn_id = 300;
        let args = slirp::UdpConnectionArgs {
            destination: local_addr,
            guest_ip: IpAddr::V4(Ipv4Addr::new(10, 0, 2, 15)),
            guest_port: 12345,
        };

        handle_native_establish_connection(
            conn_id,
            slirp::ConnectionArgs::Udp(args),
            req_tx,
            conn_writers.clone(),
        );

        let writer_tx = {
            let mut found = None;
            for _ in 0..50 {
                if let Some(tx) = conn_writers.lock().unwrap().get(&conn_id).cloned() {
                    found = Some(tx);
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            found.unwrap()
        };

        let _ = writer_tx.send(Bytes::from_static(b"hello udp"));

        let mut recv_buf = vec![0u8; 100];
        let (len, peer) = listener.recv_from(&mut recv_buf).await.unwrap();
        assert_eq!(&recv_buf[..len], b"hello udp");

        listener.send_to(b"reply udp", peer).await.unwrap();

        let req = req_rx.recv().await.unwrap();
        match req {
            slirp::SlirpRequest::Data(id, data) => {
                assert_eq!(id, conn_id);
                assert_eq!(&data[..], b"reply udp");
            }
            _ => panic!("Expected SlirpRequest::Data, got {:?}", req),
        }

        conn_writers.lock().unwrap().remove(&conn_id);
    }

    #[tokio::test]
    async fn test_native_tcp_handling() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local_addr = listener.local_addr().unwrap();

        let (req_tx, mut req_rx) = mpsc::unbounded_channel::<slirp::SlirpRequest>();
        let conn_writers = Arc::new(Mutex::new(HashMap::new()));
        let conn_id = 400;
        let args = slirp::TcpConnectionArgs {
            destination: local_addr,
            guest_ip: IpAddr::V4(Ipv4Addr::new(10, 0, 2, 15)),
            guest_port: 12345,
        };

        handle_native_establish_connection(
            conn_id,
            slirp::ConnectionArgs::Tcp(args),
            req_tx,
            conn_writers.clone(),
        );

        let (mut socket, _) = listener.accept().await.unwrap();
        let writer_tx = {
            let mut found = None;
            for _ in 0..50 {
                if let Some(tx) = conn_writers.lock().unwrap().get(&conn_id).cloned() {
                    found = Some(tx);
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            found.unwrap()
        };

        let _ = writer_tx.send(Bytes::from_static(b"hello tcp"));
        let mut recv_buf = vec![0u8; 100];
        let len = socket.read(&mut recv_buf).await.unwrap();
        assert_eq!(&recv_buf[..len], b"hello tcp");

        socket.write_all(b"reply tcp").await.unwrap();
        let req = req_rx.recv().await.unwrap();
        match req {
            slirp::SlirpRequest::Data(id, data) => {
                assert_eq!(id, conn_id);
                assert_eq!(&data[..], b"reply tcp");
            }
            _ => panic!("Expected SlirpRequest::Data, got {:?}", req),
        }

        let _ = socket.shutdown().await;
        drop(socket);
        let req = req_rx.recv().await.unwrap();
        match req {
            slirp::SlirpRequest::RemoteClosed(id) | slirp::SlirpRequest::ConnectionClosed(id) => {
                assert_eq!(id, conn_id);
            }
            _ => panic!("Expected RemoteClosed or ConnectionClosed, got {:?}", req),
        }

        conn_writers.lock().unwrap().remove(&conn_id);
    }

    #[tokio::test]
    async fn test_native_tcp_connect_failure() {
        let unused_addr = {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            listener.local_addr().unwrap()
        };

        let (req_tx, mut req_rx) = mpsc::unbounded_channel::<slirp::SlirpRequest>();
        let conn_writers = Arc::new(Mutex::new(HashMap::new()));
        let conn_id = 500;
        let args = slirp::TcpConnectionArgs {
            destination: unused_addr,
            guest_ip: IpAddr::V4(Ipv4Addr::new(10, 0, 2, 15)),
            guest_port: 12345,
        };

        handle_native_establish_connection(
            conn_id,
            slirp::ConnectionArgs::Tcp(args),
            req_tx,
            conn_writers.clone(),
        );

        let req = req_rx.recv().await.unwrap();
        match req {
            slirp::SlirpRequest::ConnectionClosed(id) => {
                assert_eq!(id, conn_id);
            }
            _ => panic!("Expected SlirpRequest::ConnectionClosed, got {:?}", req),
        }
    }

    #[tokio::test]
    async fn test_native_slirp_loop_lifecycle() {
        let (downlink_tx, _downlink_rx) = mpsc::unbounded_channel::<Bytes>();
        let (uplink_tx, uplink_rx) = mpsc::unbounded_channel::<Bytes>();

        let loop_handle =
            tokio::spawn(run_native_slirp_loop(slirp::Config::default(), downlink_tx, uplink_rx));

        uplink_tx.send(Bytes::from_static(&[0u8; 64])).unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        drop(uplink_tx);
        let result = tokio::time::timeout(Duration::from_secs(2), loop_handle).await;
        assert!(result.is_ok());
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_icmp_ipv4_checksum_and_id_rewriting() {
        let echo = etherparse::IcmpEchoHeader { id: 1, seq: 2 };
        let mut header = Icmpv4Header::new(Icmpv4Type::EchoReply(echo));
        let payload = b"hello ipv4 icmp";
        header.update_checksum(payload);

        let mut buf = header.to_bytes().to_vec();
        buf.extend_from_slice(payload);
        let guest_id = 0xABCDu16;
        let dest_ip = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
        let guest_ip = IpAddr::V4(Ipv4Addr::new(10, 0, 2, 15));

        rewrite_icmp_echo_id(&mut buf, guest_id, dest_ip, guest_ip);

        let (parsed_header, parsed_payload) = Icmpv4Header::from_slice(&buf).unwrap();
        match parsed_header.icmp_type {
            Icmpv4Type::EchoReply(echo) => assert_eq!(echo.id, 0xABCD),
            _ => panic!("Expected EchoReply"),
        }
        assert_eq!(parsed_payload, payload);
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn test_icmp_ipv6_checksum_and_id_rewriting() {
        let echo = etherparse::IcmpEchoHeader { id: 1, seq: 2 };
        let mut header = Icmpv6Header::new(Icmpv6Type::EchoReply(echo));
        let payload = b"hello ipv6 icmp";
        let dest_ip = IpAddr::V6(Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888));
        let guest_ip = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        if let (IpAddr::V6(src_v6), IpAddr::V6(dst_v6)) = (dest_ip, guest_ip) {
            let _ = header.update_checksum(src_v6.octets(), dst_v6.octets(), payload);
        }

        let mut buf = header.to_bytes().to_vec();
        buf.extend_from_slice(payload);
        let guest_id = 0x5678u16;

        rewrite_icmp_echo_id(&mut buf, guest_id, dest_ip, guest_ip);

        let (parsed_header, parsed_payload) = Icmpv6Header::from_slice(&buf).unwrap();
        match parsed_header.icmp_type {
            Icmpv6Type::EchoReply(echo) => assert_eq!(echo.id, 0x5678),
            _ => panic!("Expected EchoReply"),
        }
        assert_eq!(parsed_payload, payload);
    }
}
