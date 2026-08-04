// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_model::{ChipClient, NetsimRadioStats};
use netsim_proto::{protobuf::Message, stats::UwbApiStats};
use pdl_runtime::Packet;
use pica::packets::uci;

use crate::world::World;

#[tokio::test]
async fn test_read_statistics() {
    let mut world = World::new().await;

    // Given: A UWB chip is created
    let chip_id = 1;
    world.given_a_chip(chip_id).await;

    // When: client reads statistics
    let stats = world.client.read_statistics().await.expect("Failed to read statistics");

    // Then: stats contain the chip with zero counts
    let mut expected_stat = NetsimRadioStats::default();
    expected_stat.name = format!("uwb_chip_{}", chip_id);
    expected_stat.id = chip_id;
    expected_stat.kind = netsim_model::RadioKind::Uwb;
    let expected_stats = [expected_stat];
    assert_eq!(&*stats, &expected_stats[..]);
}

#[tokio::test]
async fn test_uwb_api_stats() {
    let mut world = World::new().await;
    let chip_id = 1;
    world.given_a_chip(chip_id).await;
    world.then_status_notification_is_received(chip_id, uci::DeviceState::DeviceStateReady).await;
    world.when_uci_is_reset(chip_id).await;

    // Initially all stats should be 0 (or not present, but default to 0)
    let stats_bytes = world.client.get_global_stats().await.unwrap().unwrap();
    let stats = UwbApiStats::parse_from_bytes(&stats_bytes).unwrap();
    assert_eq!(stats.uwb_manager.open_ranging_session(), 0);
    assert_eq!(stats.ranging_session.close(), 0);
    assert_eq!(stats.ranging_session.start(), 0);
    assert_eq!(stats.ranging_session.stop(), 0);
    assert_eq!(stats.ranging_session.reconfigure(), 0);
    assert_eq!(stats.ranging_session.session_get_app_config(), 0);
    assert_eq!(stats.ranging_session.add_controlee(), 0);
    assert_eq!(stats.ranging_session.remove_controlee(), 0);

    // 1. Trigger Open (SessionInitCmd)
    let session_id = 42;
    let init_cmd =
        uci::SessionInitCmd { session_id, session_type: uci::SessionType::FiraRangingSession };
    world.when_packet_is_sent(chip_id, &init_cmd.encode_to_vec().unwrap()).await;
    let _ = world.then_packet_is_received(chip_id).await; // Init response
    let _ = world.then_packet_is_received(chip_id).await; // Status notification (Init)

    // Verify Open stat incremented
    let stats_bytes = world.client.get_global_stats().await.unwrap().unwrap();
    let stats = UwbApiStats::parse_from_bytes(&stats_bytes).unwrap();
    assert_eq!(stats.uwb_manager.open_ranging_session(), 1);

    // 2. Trigger Reconfigure (SessionSetAppConfigCmd)
    let tlvs = vec![
        uci::AppConfigTlv {
            cfg_id: uci::AppConfigTlvType::DeviceType,
            v: vec![uci::DeviceType::Controller as u8],
        },
        uci::AppConfigTlv {
            cfg_id: uci::AppConfigTlvType::DeviceRole,
            v: vec![uci::DeviceRole::Initiator as u8],
        },
        uci::AppConfigTlv { cfg_id: uci::AppConfigTlvType::DeviceMacAddress, v: vec![0, 0] },
        uci::AppConfigTlv { cfg_id: uci::AppConfigTlvType::DstMacAddress, v: vec![0, 1] },
        uci::AppConfigTlv {
            cfg_id: uci::AppConfigTlvType::MultiNodeMode,
            v: vec![uci::MultiNodeMode::OneToOne as u8],
        },
        uci::AppConfigTlv {
            cfg_id: uci::AppConfigTlvType::RangingRoundUsage,
            v: vec![uci::RangingRoundUsage::DsTwrDeferredMode as u8],
        },
        uci::AppConfigTlv {
            cfg_id: uci::AppConfigTlvType::ScheduleMode,
            v: vec![uci::ScheduleMode::TimeScheduled as u8],
        },
        uci::AppConfigTlv {
            cfg_id: uci::AppConfigTlvType::RangingDuration,
            v: u32::MAX.to_le_bytes().to_vec(),
        },
    ];
    let config_cmd = uci::SessionSetAppConfigCmd { session_token: session_id, tlvs };
    world.when_packet_is_sent(chip_id, &config_cmd.encode_to_vec().unwrap()).await;
    let p_rsp = world.then_packet_is_received(chip_id).await; // Config response
    let (config_rsp, _) =
        uci::SessionSetAppConfigRsp::decode(&p_rsp).expect("Failed to decode config response");
    assert_eq!(config_rsp.status, uci::Status::Ok);
    let _ = world.then_packet_is_received(chip_id).await; // Status notification (Idle)

    let stats_bytes = world.client.get_global_stats().await.unwrap().unwrap();
    let stats = UwbApiStats::parse_from_bytes(&stats_bytes).unwrap();
    assert_eq!(stats.ranging_session.reconfigure(), 1);

    // 3. Trigger GetConfig (SessionGetAppConfigCmd)
    let get_config_cmd = uci::SessionGetAppConfigCmd {
        session_token: session_id,
        app_cfg: vec![uci::AppConfigTlvType::DeviceType],
    };
    world.when_packet_is_sent(chip_id, &get_config_cmd.encode_to_vec().unwrap()).await;
    let _ = world.then_packet_is_received(chip_id).await; // GetConfig response

    let stats_bytes = world.client.get_global_stats().await.unwrap().unwrap();
    let stats = UwbApiStats::parse_from_bytes(&stats_bytes).unwrap();
    assert_eq!(stats.ranging_session.session_get_app_config(), 1);

    // 3a. Trigger Add Controlee (SessionUpdateControllerMulticastListCmd with
    // ADD_CONTROLEE)
    let add_controlee_cmd = uci::SessionUpdateControllerMulticastListCmd {
        session_token: session_id,
        action: uci::UpdateMulticastListAction::AddControlee,
        payload: uci::SessionUpdateControllerMulticastListCmdPayload {
            controlees: vec![uci::Controlee { short_address: [0x12, 0x34], subsession_id: 0x5678 }],
        }
        .encode_to_vec()
        .unwrap(),
    };
    world.when_packet_is_sent(chip_id, &add_controlee_cmd.encode_to_vec().unwrap()).await;
    let _ = world.then_packet_is_received(chip_id).await; // Multicast response

    // 3a1. Trigger Add Controlee (SessionUpdateControllerMulticastListCmd with
    // ADD_CONTROLEE_WITH_SHORT_SUB_SESSION_KEY)
    let add_controlee_short_cmd = uci::SessionUpdateControllerMulticastListCmd {
        session_token: session_id,
        action: uci::UpdateMulticastListAction::AddControleeWithShortSubSessionKey,
        payload: uci::SessionUpdateControllerMulticastListCmd_2_0_16_Byte_Payload {
            controlees: vec![uci::Controlee_V2_0_16_Byte_Version {
                short_address: [0x12, 0x34],
                subsession_id: 0x5678,
                subsession_key: [0; 16],
            }],
        }
        .encode_to_vec()
        .unwrap(),
    };
    world.when_packet_is_sent(chip_id, &add_controlee_short_cmd.encode_to_vec().unwrap()).await;
    let _ = world.then_packet_is_received(chip_id).await; // Multicast response

    // 3a2. Trigger Add Controlee (SessionUpdateControllerMulticastListCmd with
    // ADD_CONTROLEE_WITH_EXTENDED_SUB_SESSION_KEY)
    let add_controlee_ext_cmd = uci::SessionUpdateControllerMulticastListCmd {
        session_token: session_id,
        action: uci::UpdateMulticastListAction::AddControleeWithExtendedSubSessionKey,
        payload: uci::SessionUpdateControllerMulticastListCmd_2_0_32_Byte_Payload {
            controlees: vec![uci::Controlee_V2_0_32_Byte_Version {
                short_address: [0x12, 0x34],
                subsession_id: 0x5678,
                subsession_key: [0; 32],
            }],
        }
        .encode_to_vec()
        .unwrap(),
    };
    world.when_packet_is_sent(chip_id, &add_controlee_ext_cmd.encode_to_vec().unwrap()).await;
    let _ = world.then_packet_is_received(chip_id).await; // Multicast response

    let stats_bytes = world.client.get_global_stats().await.unwrap().unwrap();
    let stats = UwbApiStats::parse_from_bytes(&stats_bytes).unwrap();
    assert_eq!(stats.ranging_session.add_controlee(), 3);

    // 3b. Trigger Remove Controlee (SessionUpdateControllerMulticastListCmd with
    // REMOVE_CONTROLEE)
    let remove_controlee_cmd = uci::SessionUpdateControllerMulticastListCmd {
        session_token: session_id,
        action: uci::UpdateMulticastListAction::RemoveControlee,
        payload: uci::SessionUpdateControllerMulticastListCmdPayload {
            controlees: vec![uci::Controlee { short_address: [0x12, 0x34], subsession_id: 0x5678 }],
        }
        .encode_to_vec()
        .unwrap(),
    };
    world.when_packet_is_sent(chip_id, &remove_controlee_cmd.encode_to_vec().unwrap()).await;
    let _ = world.then_packet_is_received(chip_id).await; // Multicast response

    let stats_bytes = world.client.get_global_stats().await.unwrap().unwrap();
    let stats = UwbApiStats::parse_from_bytes(&stats_bytes).unwrap();
    assert_eq!(stats.ranging_session.remove_controlee(), 1);

    // 4. Trigger Start (SessionStartCmd)
    let start_cmd = uci::SessionStartCmd { session_id };
    world.when_packet_is_sent(chip_id, &start_cmd.encode_to_vec().unwrap()).await;
    let _ = world.then_packet_is_received(chip_id).await; // Start response
    let _ = world.then_packet_is_received(chip_id).await; // Status notification (Active)
    let _ = world.then_packet_is_received(chip_id).await; // Device Status notification (Active)

    let stats_bytes = world.client.get_global_stats().await.unwrap().unwrap();
    let stats = UwbApiStats::parse_from_bytes(&stats_bytes).unwrap();
    assert_eq!(stats.ranging_session.start(), 1);

    // 5. Trigger Stop (SessionStopCmd)
    let stop_cmd = uci::SessionStopCmd { session_id };
    world.when_packet_is_sent(chip_id, &stop_cmd.encode_to_vec().unwrap()).await;
    let _ = world.then_packet_is_received(chip_id).await; // Stop response
    let _ = world.then_packet_is_received(chip_id).await; // Status notification (Idle)
    let _ = world.then_packet_is_received(chip_id).await; // Device Status notification (Ready)

    let stats_bytes = world.client.get_global_stats().await.unwrap().unwrap();
    let stats = UwbApiStats::parse_from_bytes(&stats_bytes).unwrap();
    assert_eq!(stats.ranging_session.stop(), 1);

    // 6. Trigger Close (SessionDeinitCmd)
    let deinit_cmd = uci::SessionDeinitCmd { session_token: session_id };
    world.when_packet_is_sent(chip_id, &deinit_cmd.encode_to_vec().unwrap()).await;
    let _ = world.then_packet_is_received(chip_id).await; // Deinit response

    let stats_bytes = world.client.get_global_stats().await.unwrap().unwrap();
    let stats = UwbApiStats::parse_from_bytes(&stats_bytes).unwrap();
    assert_eq!(stats.ranging_session.close(), 1);
}
