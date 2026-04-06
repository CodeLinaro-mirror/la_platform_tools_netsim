// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

use device_actor::DeviceClient;
use futures::StreamExt;
use netsim_model::{BluetoothCreate, BluetoothMode, ChipKindParams, DeviceParams};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
};
use tokio_util::codec::FramedRead;
use tracing::{info, warn};

use crate::h4::H4Codec;

/// Start the async TCP transport for HCI connections
///
/// Binds to `hci_port` and loops, passing each accepted connection to
/// `handle_hci_client`.
pub async fn run(hci_port: u16, device_client: DeviceClient) {
    let listener = match TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, hci_port))).await
    {
        Ok(l) => l,
        Err(e) => {
            warn!(
                    "Failed to bind to 127.0.0.1:{hci_port} in HCI socket server, trying [::1]:{hci_port}: {e}"
                );
            match TcpListener::bind(SocketAddr::from((Ipv6Addr::LOCALHOST, hci_port))).await {
                Ok(l) => l,
                Err(e) => {
                    warn!("Failed to start HCI socket server: {e}");
                    return;
                }
            }
        }
    };

    info!("Hci socket server is listening on: {hci_port}");

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                info!("Hci client address: {addr}");
                let dc_clone = device_client.clone();
                tokio::spawn(async move {
                    handle_hci_client(stream, addr, dc_clone).await;
                });
            }
            Err(e) => {
                warn!("Error accepting HCI connection: {e}");
            }
        }
    }
}

async fn handle_hci_client(stream: TcpStream, addr: SocketAddr, device_client: DeviceClient) {
    let (socket_rx, mut socket_tx) = stream.into_split();

    // 1. Setup the AsyncStream bridging to `packet_stream::PacketStream`
    let (stream_tx, stream_rx) =
        tokio::sync::mpsc::channel::<Result<bytes::Bytes, std::io::Error>>(100);
    // 2. Setup the AsyncSink bridging from `packet_stream::PacketSink`
    let (sink_tx, mut sink_rx) = tokio::sync::mpsc::channel::<bytes::Bytes>(100);

    // Read Task: Read H4 packets from the socket and send to `stream_tx`
    let read_addr = addr;
    tokio::spawn(async move {
        let mut framed = FramedRead::new(socket_rx, H4Codec);
        while let Some(packet_result) = framed.next().await {
            match packet_result {
                Ok(packet) => {
                    if stream_tx.send(Ok(packet)).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    warn!("Error reading from HCI socket {read_addr}: {e:?}");
                    break;
                }
            }
        }
        info!("End socket reader connection with {read_addr}.");
    });

    // Write Task: Receive payloads from `sink_rx` and write them to the socket
    tokio::spawn(async move {
        while let Some(bytes) = sink_rx.recv().await {
            if socket_tx.write_all(&bytes).await.is_err() {
                break;
            }
        }
    });

    // 3. Create the Chip parameters
    let name = format!("socket-{addr}");
    let chip_create_params = BluetoothCreate {
        address: String::new(), // Address will be automatically derived or configured
        bt_properties: netsim_model::Controller::default(),
        mode: BluetoothMode::Device(DeviceParams {}),
    };

    let chip_config = netsim_model::ChipConfig {
        name: name.clone(),
        manufacturer: "Google".to_string(),
        product_name: "Google".to_string(),
        chip_kind_params: ChipKindParams::Bluetooth(chip_create_params),
    };

    let packet_stream: netsim_model::PacketStream = Box::new(
        tokio_stream::wrappers::ReceiverStream::new(stream_rx)
            .filter_map(|res| std::future::ready(res.ok())),
    );
    let packet_sink: netsim_model::PacketSink = Box::pin(futures::sink::unfold(
        sink_tx,
        |tx: tokio::sync::mpsc::Sender<bytes::Bytes>, item: bytes::Bytes| async move {
            tx.send(item).await.map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Failed to send to sink_tx")
            })?;
            Ok(tx)
        },
    ));

    let request = device_api::DeviceAddChip {
        device_guid: name.clone(),
        packet_stream: Some(packet_stream),
        packet_sink: Some(packet_sink),
        device_config: device_api::DeviceConfig {
            name: name,
            visible: true,
            pose: Default::default(),
            builtin: false,
            device_info: None,
        },
        chip_config,
    };

    // 4. Send request to add chip
    if let Err(e) = device_client.add_chip(request).await {
        warn!("Failed to add virtual HCI chip for {addr}: {e}");
    }
}
