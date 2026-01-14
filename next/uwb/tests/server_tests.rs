// Copyright 2024-2025 The Android Open Source Project

use netsim_model::{
    chip::{ChipCreate, ChipId, ChipKind, NetworkParams, UwbCreate},
    chip_error::ChipError,
    device::DeviceId,
    devices::DeviceClient,
};
use tokio::sync::mpsc;
use uwb::Server;

// Helper to create a default ChipCreate for UWB
fn create_uwb_params(id: u32) -> ChipCreate {
    let (packet_tx, packet_rx) = mpsc::unbounded_channel();
    let (sink_tx, sink_rx) = mpsc::unbounded_channel();

    ChipCreate {
        id: ChipId(id),
        packet_stream: Some(Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(
            packet_rx,
        ))),
        packet_sink: Some(Box::pin(futures::sink::unfold(sink_tx, |tx, bytes| async move {
            tx.send(bytes)
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "send error"))?;
            Ok(tx)
        }))),
        config: netsim_model::chip::ChipConfig {
            model: ChipModel {
                name: format!("uwb_chip_{}", id),
                manufacturer: "Netsim".to_string(),
                product_name: "TestUwb".to_string(),
            },
            network_params: NetworkParams::Uwb(UwbCreate {}),
        },
        device_id: DeviceId(1),
    }
}

async fn setup() -> (Server, netsim_model::chip::RadioChipClient) {
    let device_client = DeviceClient::new(mpsc::channel(10).0);
    Server::new(device_client)
}

#[tokio::test]
async fn test_create_and_get_chip() -> Result<(), Box<dyn std::error::Error>> {
    let (server, client) = setup().await;
    tokio::spawn(async move { server.run().await });

    let chip_id = ChipId(1);
    let params = create_uwb_params(chip_id.0);
    let model = params.config.model.clone();

    // Create a chip
    client.create(params).await??;

    // Get the chip
    let chip_info = client.read(chip_id).await??;

    match chip_info {
        ChipInfo::Uwb(chip) => {
            assert_eq!(chip.id, chip_id.0);
            assert_eq!(chip.kind, ChipKind::UWB);
            assert_eq!(chip.model, model);
        }
        _ => panic!("Unexpected ChipInfo variant"),
    }
    Ok(())
}

#[tokio::test]
async fn test_create_duplicate_chip() -> Result<(), Box<dyn std::error::Error>> {
    let (server, client) = setup().await;
    tokio::spawn(async move { server.run().await });

    let chip_id = ChipId(2);
    client.create(create_uwb_params(chip_id.0)).await??;

    // Try creating again
    let result = client.create(create_uwb_params(chip_id.0)).await?;
    match result {
        Err(ChipError::ChipAlreadyExists(id)) => {
            assert_eq!(id, chip_id);
        }
        _ => panic!("Expected ChipAlreadyExists error, got {:?}", result),
    }
    Ok(())
}

#[tokio::test]
async fn test_get_chip_not_found() -> Result<(), Box<dyn std::error::Error>> {
    let (server, client) = setup().await;
    tokio::spawn(async move { server.run().await });

    let chip_id = ChipId(99);
    let result = client.read(chip_id).await?;

    match result {
        Err(ChipError::ChipNotFound(id)) => {
            assert_eq!(id, chip_id);
        }
        _ => panic!("Expected ChipNotFound error, got {:?}", result),
    }
    Ok(())
}

#[tokio::test]
async fn test_delete_chip() -> Result<(), Box<dyn std::error::Error>> {
    let (server, client) = setup().await;
    tokio::spawn(async move { server.run().await });

    let chip_id = ChipId(3);
    client.create(create_uwb_params(chip_id.0)).await??;

    // Delete the chip
    client.delete(chip_id).await??;

    // Try to get the chip, should be NotFound
    let result = client.read(chip_id).await?;
    match result {
        Err(ChipError::ChipNotFound(id)) => {
            assert_eq!(id, chip_id);
        }
        _ => panic!("Expected ChipNotFound after delete, got {:?}", result),
    }
    Ok(())
}
