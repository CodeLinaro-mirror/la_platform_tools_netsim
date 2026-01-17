// Copyright 2024-2025 The Android Open Source Project

use netsim_model::{
    chip::{ChipClient, ChipId},
    chip_error::ChipError,
    client_error::ClientError,
};
use tokio::sync::mpsc;
use wifi::Server;

async fn setup() -> (Server, netsim_model::chip::RadioChipClient) {
    let (device_tx, _) = mpsc::channel(10);
    let resource_client = actor_framework::ResourceClient::new(device_tx);
    let device_client = client::DeviceClient::new(Box::new(resource_client));
    Server::new(device_client)
}

// TODO: add tests for create and delete

#[tokio::test]
async fn test_get_chip_not_found() -> Result<(), Box<dyn std::error::Error>> {
    test_get_chip_not_found_inner().await
}

async fn test_get_chip_not_found_inner() -> Result<(), Box<dyn std::error::Error>> {
    let (server, client) = setup().await;
    tokio::spawn(async move { server.run().await });

    let chip_id = ChipId(99);
    let result = client.read(chip_id).await;

    match result {
        Err(ClientError::Chip(ChipError::ChipNotFound(id))) => {
            assert_eq!(id, chip_id);
        }
        _ => panic!("Expected ChipNotFound error, got {:?}", result),
    }
    Ok(())
}
