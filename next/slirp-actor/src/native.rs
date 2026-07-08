// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use bytes::Bytes;
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

fn handle_native_establish_connection(
    conn_id: u64,
    args: slirp::ConnectionArgs,
    req_tx: mpsc::UnboundedSender<slirp::SlirpRequest>,
    conn_writers: Arc<Mutex<HashMap<u64, mpsc::UnboundedSender<Bytes>>>>,
) {
    match args {
        slirp::ConnectionArgs::Tcp(tcp_args) => {
            let (writer_tx, mut writer_rx) = mpsc::unbounded_channel::<Bytes>();
            conn_writers.lock().unwrap().insert(conn_id, writer_tx);
            let dest = tcp_args.destination;
            tokio::spawn(async move {
                match TcpStream::connect(dest).await {
                    Ok(stream) => {
                        let (mut reader, mut writer) = stream.into_split();
                        let req_tx_clone = req_tx.clone();
                        let read_task = tokio::spawn(async move {
                            let mut buf = [0u8; 8192];
                            loop {
                                match reader.read(&mut buf).await {
                                    Ok(0) => {
                                        let _ = req_tx_clone
                                            .send(slirp::SlirpRequest::RemoteClosed(conn_id));
                                        break;
                                    }
                                    Ok(n) => {
                                        let _ = req_tx_clone.send(slirp::SlirpRequest::Data(
                                            conn_id,
                                            Bytes::copy_from_slice(&buf[..n]),
                                        ));
                                    }
                                    Err(_) => {
                                        let _ = req_tx_clone
                                            .send(slirp::SlirpRequest::ConnectionClosed(conn_id));
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
                    Err(e) => {
                        info!("Failed to connect TCP to {dest}: {e}");
                        let _ = req_tx.send(slirp::SlirpRequest::ConnectionClosed(conn_id));
                    }
                }
            });
        }
        slirp::ConnectionArgs::Udp(udp_args) => {
            let (writer_tx, mut writer_rx) = mpsc::unbounded_channel::<Bytes>();
            conn_writers.lock().unwrap().insert(conn_id, writer_tx);
            let dest = udp_args.destination;
            tokio::spawn(async move {
                let bind_addr = match dest {
                    std::net::SocketAddr::V4(_) => "0.0.0.0:0",
                    std::net::SocketAddr::V6(_) => "[::]:0",
                };
                if let Ok(socket) = UdpSocket::bind(bind_addr).await {
                    if socket.connect(dest).await.is_ok() {
                        let socket = Arc::new(socket);
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
                                        let _ = req_tx_clone
                                            .send(slirp::SlirpRequest::ConnectionClosed(conn_id));
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
                    } else {
                        let _ = req_tx.send(slirp::SlirpRequest::ConnectionClosed(conn_id));
                    }
                } else {
                    let _ = req_tx.send(slirp::SlirpRequest::ConnectionClosed(conn_id));
                }
            });
        }
        _ => {}
    }
}
