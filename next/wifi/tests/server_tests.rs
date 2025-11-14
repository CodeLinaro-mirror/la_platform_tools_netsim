// Copyright 2024-2025 The Android Open Source Project

use netsim_api::{
    chip_error::ChipError, chips::ChipId, client_error::ClientError, devices::DeviceClient,
};
use tokio::sync::mpsc;
use wifi::Server;

async fn setup() -> (Server, netsim_api::chips::ChipClient) {
    let device_client = DeviceClient::new(mpsc::channel(10).0);
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
