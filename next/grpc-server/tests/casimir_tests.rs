// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use grpc_server::casimir::CasimirControlServiceImpl;
    use grpcio::{ChannelBuilder, Environment, ServerBuilder, ServerCredentials};
    use netsim_proto::{
        casimir_control::{SendBroadcastRequest, Void},
        casimir_control_grpc::CasimirControlServiceClient,
    };
    use pdl_runtime::Packet;
    use tracing::info;

    // Helper to start the actor framework and return the nfc_client and
    // scene_client
    async fn setup_actors() -> (nfc_actor::NfcClient, Option<nfc_actor::SceneClient>) {
        // 1. Device Actor (mock/dummy client only, we do NOT run the actor runner)
        let (_device_actor_runner, device_client) = device_actor::new();

        // 2. NFC Actor with in-process Casimir started
        let (nfc_actor, nfc_client) = nfc_actor::new();
        let mut nfc_actor_impl = nfc_actor::NfcActor::new(device_client);
        nfc_actor_impl.start_casimir();
        let scene_client = nfc_actor_impl.scene_client.clone();

        tokio::spawn(async move {
            // Spawn NFC actor runner
            nfc_actor.run(nfc_actor_impl).await;
        });

        (nfc_client, scene_client)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_casimir_control_service_lifecycle() {
        // 1. Start backend actors
        let (nfc_client, scene_client) = setup_actors().await;
        let scene_client = scene_client.expect("Casimir Scene client not initialized");

        // 2. Add a virtual RF-level mock tag to the Casimir scene
        // This tag will automatically respond to WUPA (PollCommand) and ISO-DEP Select
        // commands, allowing PollA to succeed instantly without requiring NCI guest
        // initialization.
        let (rf_tx, mut rf_rx) = tokio::sync::mpsc::unbounded_channel();
        let add_result = scene_client
            .add_device(move |id, scene_rf_tx| {
                nfc_actor::casimir::Device {
                    id,
                    rf_tx,
                    task: Box::pin(async move {
                        use nfc_actor::casimir::packets::rf;

                        while let Some(packet) = rf_rx.recv().await {
                            let bytes = packet.encode_to_vec().unwrap();
                            info!("MockTag: Received packet bytes (len={}): {:02x?}", bytes.len(), bytes);

                            // Manual routing for ISO-DEP Select Command to bypass PDL specialize() TrailingBytes bug
                            if packet.packet_type() == rf::RfPacketType::SelectCommand
                                && packet.technology() == rf::Technology::NfcA
                                && packet.protocol() == rf::Protocol::IsoDep
                            {
                                match rf::T4ATSelectCommand::decode_full(&bytes) {
                                    Ok(select_cmd) => {
                                        info!("MockTag: Received T4ATSelectCommand (decoded directly) from {}", select_cmd.sender());
                                        let resp = rf::T4ATSelectResponse {
                                            sender: id,
                                            receiver: select_cmd.sender(),
                                            bitrate: rf::BitRate::BitRate106KbitS,
                                            power_level: 10,
                                            rats_response: vec![0x06, 0x70, 0x80, 0x10, 0x20, 0x30],
                                        };
                                        let rf_packet = resp.try_into().unwrap();
                                        let _ = scene_rf_tx.send(rf_packet);
                                        continue;
                                    }
                                    Err(e) => {
                                        tracing::error!("MockTag: Failed to decode T4ATSelectCommand: {:?}", e);
                                    }
                                }
                            }

                            // Fallback to specialize for other packets (like PollCommand)
                            let specialized = packet.specialize();
                            info!("MockTag: Specialized packet: {:?}", specialized);
                            if let Ok(rf::RfPacketChild::PollCommand(poll_cmd)) = specialized {
                                info!("MockTag: Received PollCommand from {}", poll_cmd.sender());
                                let resp = rf::NfcAPollResponse {
                                    sender: id,
                                    receiver: poll_cmd.sender(),
                                    protocol: rf::Protocol::IsoDep,
                                    bitrate: rf::BitRate::BitRate106KbitS,
                                    power_level: 10,
                                    nfcid1: vec![0x01, 0x02, 0x03, 0x04],
                                    int_protocol: 1, // Type 4A Tag Platform (ISO-DEP)
                                    bit_frame_sdd: 0x04,
                                };
                                let rf_packet = resp.try_into().unwrap();
                                let _ = scene_rf_tx.send(rf_packet);
                            }
                        }
                        Ok(())
                    }),
                }
            })
            .await;

        assert!(add_result.is_ok());
        let mock_device_id = add_result.unwrap();
        assert_eq!(mock_device_id, 0); // First device in scene gets ID 0 (0-based)

        // 3. Start local gRPC Server on a free port
        let service = CasimirControlServiceImpl::new(nfc_client);
        let env = Arc::new(Environment::new(1));
        let casimir_service =
            netsim_proto::casimir_control_grpc::create_casimir_control_service(service);

        let mut server =
            ServerBuilder::new(env.clone()).register_service(casimir_service).build().unwrap();

        let port = server.add_listening_port("localhost:0", ServerCredentials::insecure()).unwrap();
        server.start();
        info!("Test gRPC server listening on localhost:{}", port);

        // 4. Create gRPC Client connecting to our local server
        let ch = ChannelBuilder::new(env).connect(&format!("localhost:{port}"));
        let client = CasimirControlServiceClient::new(ch);

        // 5. Execute Test Cases

        // Test Case A: Init the control channel
        let init_res = client.init(&Void::new());
        assert!(init_res.is_ok(), "Init failed: {:?}", init_res.err());

        // Test Case B: PollA (should discover our mock tag instantly)
        let poll_res = client.poll_a(&Void::new());
        assert!(poll_res.is_ok(), "PollA failed: {:?}", poll_res.err());
        let sender_id = poll_res.unwrap().sender_id;
        // Verify 1-based ID workaround: internal mock ID (0) + 1 = 1
        assert_eq!(sender_id, 1);

        // Test Case C: SendBroadcast (inject RF packet)
        let mut bcast_req = SendBroadcastRequest::new();
        bcast_req.data = "00112233".to_string(); // Hex string payload
        let bcast_res = client.send_broadcast(&bcast_req);
        assert!(bcast_res.is_ok(), "SendBroadcast failed: {:?}", bcast_res.err());

        // Test Case D: Close the control channel
        let close_res = client.close(&Void::new());
        assert!(close_res.is_ok(), "Close failed: {:?}", close_res.err());

        // 6. Cleanup
        let _ = server.shutdown().await;
    }
}
