// Copyright 2026 The Android Open Source Project

use std::sync::atomic::{AtomicU64, Ordering};

use bytes::Bytes;
use device_actor::DeviceClient;
use futures::SinkExt;
use grpcio::{RpcContext, ServerStreamingSink, WriteFlags};
use netsim_model::{
    chip::{BluetoothCreate, BluetoothMode, ChipConfig, ChipKindParams, ScannerParams},
    device::{DeviceAddChip, DeviceConfig, Position},
};
use netsim_proto::{
    ble_service::{ScanRequest, ScanResponse},
    ble_service_grpc::BleService,
};
use tokio::sync::mpsc;
use tracing::{error, warn};

static SCANNER_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

struct ScannerGuard {
    client: DeviceClient,
    device_id: device_api::DeviceId,
}

impl Drop for ScannerGuard {
    fn drop(&mut self) {
        let client = self.client.clone();
        let id = self.device_id;
        tokio::spawn(async move {
            if let Err(e) = client.delete(id).await {
                error!("Failed to delete scanner device {}: {}", id.0, e);
            }
        });
    }
}

#[derive(Clone)]
pub struct BleServiceImpl {
    device_client: DeviceClient,
}

impl BleServiceImpl {
    pub fn new(device_client: DeviceClient) -> Self {
        Self { device_client }
    }
}

impl BleService for BleServiceImpl {
    fn scan(
        &mut self,
        ctx: RpcContext<'_>,
        req: ScanRequest,
        mut stream_sink: ServerStreamingSink<ScanResponse>,
    ) {
        let device_client = self.device_client.clone();

        ctx.spawn(async move {
            let (packet_tx, mut packet_rx) = mpsc::channel::<Bytes>(100);

            let app_sink: netsim_model::chip::PacketSink =
                Box::pin(futures::sink::unfold(packet_tx, |tx, item: Bytes| async move {
                    tx.send(item).await.map_err(|e| {
                        std::io::Error::new(std::io::ErrorKind::BrokenPipe, e.to_string())
                    })?;
                    Ok(tx)
                }));

            let position = req
                .position
                .as_ref()
                .map(|p| Position { x: p.x, y: p.y, z: p.z })
                .unwrap_or_default();

            let id = SCANNER_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
            let guid = format!("BleScanner-{}", id);

            let device_config = DeviceConfig {
                name: guid.clone(),
                visible: true,
                position,
                orientation: Default::default(),
                builtin: true,
                device_info: None,
            };

            let chip_config = ChipConfig {
                name: "BleScanner".to_string(),
                manufacturer: "Netsim".to_string(),
                product_name: "Scanner".to_string(),
                chip_kind_params: ChipKindParams::Bluetooth(BluetoothCreate {
                    address: "".to_string(), // will be generated
                    bt_properties: Default::default(),
                    mode: BluetoothMode::Scanner(ScannerParams::default()),
                }),
            };

            let add_chip = DeviceAddChip {
                device_guid: guid,
                packet_stream: None,
                packet_sink: Some(app_sink),
                device_config,
                chip_config,
            };

            let device_id = match device_client.add_chip(add_chip).await {
                Ok(id) => id,
                Err(e) => {
                    error!("Failed to create scanner chip: {}", e);
                    return;
                }
            };

            let _guard = ScannerGuard { client: device_client.clone(), device_id };

            // Forward packets from mpsc rx -> grpc stream
            while let Some(packet) = packet_rx.recv().await {
                let mut resp = ScanResponse::new();
                resp.packet = packet.to_vec();
                if let Err(e) = stream_sink.send((resp, WriteFlags::default())).await {
                    warn!("Scan stream disconnected or error: {}", e);
                    break;
                }
            }
            let _ = stream_sink.close().await;
        });
    }
}
