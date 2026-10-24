// Copyright 2026 The Android Open Source Project
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::net::{IpAddr, Ipv4Addr};

use netsim_model::{CellNetworkConfig, Quirks, RegistrationStatus};

use crate::world::{SLIRP_FIRST_V4, SLIRP_FIRST_V6, World};

#[tokio::test]
async fn test_create_chip() {
    // Scenario: Creating a cellular chip initializes an active idle modem in the actor
    // Given a cell actor with no chips
    let mut world = World::new().await;

    // When cellular chip 1 is created
    world.given_cell_chip(1).await;

    // Then reading and listing the cellular chip report an enabled idle modem
    world.then_cell_state_is(1, "idle").await;
    world.then_listed_cell_states_are(&[(1, true, "idle")]).await;
}

#[tokio::test]
async fn test_delete_chip() {
    // Scenario: Deleting a cellular chip removes it and notifies the device actor
    let mut world = World::new().await;

    // Given an active cellular chip 2
    world.given_cell_chip(2).await;

    // When chip 2 is deleted
    world.when_delete_chip(2).await;

    // Then the device actor is notified that chip 2 was removed
    world.then_device_notified_chip_removed(2).await;
}

#[tokio::test]
async fn test_stream_error_triggers_delete() {
    // Scenario: Closing the guest packet stream triggers automatic chip deletion
    let mut world = World::new().await;

    // Given an active cellular chip 3
    world.given_cell_chip(3).await;

    // When the guest closes its packet stream for chip 3
    world.when_guest_stream_closes(3);

    // Then the device actor is notified that chip 3 was removed
    world.then_device_notified_chip_removed(3).await;
}

#[tokio::test]
async fn test_sink_error_triggers_delete() {
    // Scenario: A host sink error event removes the cellular chip and notifies the device actor
    let mut world = World::new().await;

    // Given an active cellular chip 4 in the idle state
    world.given_cell_chip(4).await;
    world.then_cell_state_is(4, "idle").await;

    // When the modem simulator emits a sink error for chip 4
    world.when_host_emits_sink_error(4);

    // Then the device actor is notified and chip 4 is no longer found
    world.then_device_notified_chip_removed(4).await;
    world.then_chip_not_found(4).await;
}

#[tokio::test]
async fn test_stream_to_controller_echo() {
    // Scenario: Sending an AT command over the guest packet stream returns the modem response on the guest sink
    let mut world = World::new().await;

    // Given an active cellular chip 1
    world.given_cell_chip(1).await;

    // When the guest sends a basic AT command over its packet stream
    world.when_guest_sends_packet(1, b"AT\r").await;

    // Then the modem controller processes the command and replies with OK on the guest sink
    world.then_guest_receives_exact_packet(1, "OK\r\n").await;
}

#[tokio::test]
async fn test_get_chip() {
    // Scenario: Reading an existing cellular chip returns its state while reading an unknown ID returns ChipNotFound
    let mut world = World::new().await;

    // Given an active cellular chip 1
    world.given_cell_chip(1).await;

    // When chip 1 and non-existent chip 99 are read
    // Then reading chip 1 returns idle and reading chip 99 returns ChipNotFound
    world.then_cell_state_is(1, "idle").await;
    world.then_chip_not_found(99).await;
}

#[tokio::test]
async fn test_get_and_list_chip_states_across_idle_ringing_and_down() {
    // Scenario: Reading and listing a cellular chip reflect transitions across idle, ringing, and down states
    let mut world = World::new().await;

    // Given a voice-registered cellular chip 1 in the idle state
    world.given_cell_chip(1).await;
    world.given_voice_registered_home(1).await;
    world.then_cell_state_is(1, "idle").await;
    world.then_listed_cell_states_are(&[(1, true, "idle")]).await;

    // When an incoming call arrives on chip 1
    world.when_incoming_call(1, "123456").await;

    // Then both read and list report the ringing state
    world.then_cell_state_is(1, "ringing").await;
    world.then_listed_cell_states_are(&[(1, true, "ringing")]).await;

    // When chip 1 is disabled
    world.when_set_chip_enabled(1, false).await;

    // Then both read and list report the down state with radio disabled
    world.then_cell_state_is(1, "down").await;
    world.then_listed_cell_states_are(&[(1, false, "down")]).await;
}

#[tokio::test]
async fn test_delete_chip_non_blocking() {
    // Scenario: Deleting a chip spawns device notification asynchronously without blocking concurrent reads
    let mut world = World::new().await;

    // Given an active cellular chip 2
    world.given_cell_chip(2).await;

    // When chip deletion is initiated while the device actor has not yet acknowledged removal
    world.when_start_delete_chip(2);
    world.then_device_receives_chip_removed_unacked(2).await;

    // Then concurrent read requests on the cell actor do not block and deletion completes once acknowledged
    world.then_read_does_not_block(2).await;
    world.when_device_acks_chip_removed();
    world.then_pending_delete_succeeds().await;
}

#[tokio::test]
async fn test_get_chip_status_fields() {
    // Scenario: Updating signal strength and registration status is reflected in cellular chip status fields
    let mut world = World::new().await;

    // Given a newly created cellular chip 1 with default signal strength and unregistered state
    world.given_cell_chip(1).await;
    world.then_cell_sms_count_is(1, 0).await;
    // Default RSSI in NetworkService::new is 20, default BER is 99
    world
        .then_cell_status_is(
            1,
            20,
            99,
            RegistrationStatus::NotRegistered,
            RegistrationStatus::NotRegistered,
        )
        .await;

    // When signal strength and voice/data registrations are updated
    world.when_set_signal_strength(1, 15, 2).await;
    world
        .when_set_registrations(1, RegistrationStatus::RegisteredHome, RegistrationStatus::Roaming)
        .await;

    // Then reading chip 1 reflects the updated RSSI, BER, and registration statuses
    world
        .then_cell_status_is(
            1,
            15,
            2,
            RegistrationStatus::RegisteredHome,
            RegistrationStatus::Roaming,
        )
        .await;
}

#[tokio::test]
async fn test_receive_pdu_action_forwarding() {
    // Scenario: Forwarding a valid hex SMS PDU delivers a +CMT URC to the guest sink
    let mut world = World::new().await;

    // Given a voice-registered cellular chip 1
    world.given_cell_chip(1).await;
    world.given_voice_registered_home(1).await;

    // When a valid hex PDU is injected via ReceivePdu
    world.when_receive_pdu(1, "0011000B915155255155F40000AA01F0").await;

    // Then the guest sink receives the formatted +CMT URC with the PDU payload
    world
        .then_guest_receives_exact_packet(1, "+CMT: ,15\r\n0011000B915155255155F40000AA01F0\r\n")
        .await;
}

#[tokio::test]
async fn test_receive_sms_action_forwarding() {
    // Scenario: Forwarding an incoming text SMS encodes and delivers a UCS-2 PDU +CMT URC to the guest sink
    let mut world = World::new().await;

    // Given a voice-registered cellular chip 1
    world.given_cell_chip(1).await;
    world.given_voice_registered_home(1).await;

    // When an incoming text SMS is injected via ReceiveSms
    world.when_receive_sms(1, "123456", "hello").await;

    // Then the guest sink receives a +CMT URC containing the sender and UCS-2 encoded message
    world
        .then_guest_receives_lowercase_packet_framing(
            1,
            "+cmt: ,26\r\n002406812143650008",
            "0a00680065006c006c006f\r\n",
        )
        .await;
}

#[tokio::test]
async fn test_receive_pdu_invalid_hex() {
    // Scenario: Rejecting a ReceivePdu action when the PDU contains non-hexadecimal characters
    let mut world = World::new().await;

    // Given an active cellular chip 1
    world.given_cell_chip(1).await;

    // When ReceivePdu is called with an invalid hex string
    world.when_try_receive_pdu(1, "0011000B915155255155F40000AA01Fg").await;

    // Then the action fails with an Invalid PDU format error
    world.then_last_error_contains("Invalid PDU format");
}

#[tokio::test]
async fn test_incoming_call_invalid_number() {
    // Scenario: Rejecting an IncomingCall action when the phone number contains invalid characters
    let mut world = World::new().await;

    // Given an active cellular chip 1
    world.given_cell_chip(1).await;

    // When IncomingCall is called with alphabetic characters in the phone number
    world.when_try_incoming_call(1, "12345abc").await;

    // Then the action fails with an Invalid phone number error
    world.then_last_error_contains("Invalid phone number");
}

#[tokio::test]
async fn test_network_config_lifecycle_and_update() {
    // Scenario: Creating a cellular chip with initial network configs and updating them via ChipVariantUpdate::Cell
    let initial_config = CellNetworkConfig {
        ip_address: IpAddr::V4(Ipv4Addr::new(192, 168, 97, 2)),
        prefixlen: 30,
        gateway: IpAddr::V4(Ipv4Addr::new(192, 168, 97, 1)),
        dns: IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
    };
    let updated_config = CellNetworkConfig {
        ip_address: IpAddr::V4(Ipv4Addr::new(192, 168, 97, 6)),
        prefixlen: 30,
        gateway: IpAddr::V4(Ipv4Addr::new(192, 168, 97, 5)),
        dns: IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),
    };

    let mut world = World::new().await;

    // Given a cellular chip 1 created with an initial IPv4 network config
    world.given_cell_chip_with_configs(1, vec![initial_config.clone()]).await;
    world.then_cell_network_configs_are(1, &[initial_config]).await;

    // When the chip's network configs are updated
    world.when_update_network_configs(1, vec![updated_config.clone()]).await;

    // Then reading chip 1 returns the updated IPv4 network config
    world.then_cell_network_configs_are(1, &[updated_config]).await;
}

#[tokio::test]
async fn test_goldfish_leases_and_releases_slirp_ip() {
    // Scenario: Goldfish dynamic SLIRP lease allocates a dual-stack IP on create and releases it on delete
    let goldfish_quirks = Quirks { dynamic_slirp_lease: true, ..Default::default() };
    let mut world = World::new().await;

    // Given a cellular chip 1 with dynamic_slirp_lease enabled and no pre-provisioned configs
    world.given_cell_chip_with_quirks(1, goldfish_quirks, Vec::new()).await;

    // Then chip 1 receives the first dual-stack SLIRP lease
    world.then_cell_has_dual_stack_lease(1, SLIRP_FIRST_V4, SLIRP_FIRST_V6).await;

    // When chip 1 is deleted
    world.when_delete_chip(1).await;

    // Then the released SLIRP IPv4 lease is recycled to the next requester
    world.then_slirp_allocates_ipv4_lease(2, SLIRP_FIRST_V4).await;
}

#[tokio::test]
async fn test_cuttlefish_and_provided_configs_do_not_lease_slirp_ip() {
    // Scenario: Cuttlefish modems and modems with explicit network configs do not consume SLIRP leases
    // Given a cell actor and a custom IPv4 network configuration
    let cuttlefish_quirks = Quirks { is_cuttlefish: true, ..Default::default() };
    let custom_config = vec![CellNetworkConfig {
        ip_address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10)),
        prefixlen: 24,
        gateway: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
        dns: IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
    }];

    let mut world = World::new().await;

    // When chips 10, 15, and 20 are created with Cuttlefish quirks, default quirks, and explicit network configs
    world.given_cell_chip_with_quirks(10, cuttlefish_quirks, Vec::new()).await;
    world.then_cell_network_configs_are(10, &[]).await;

    world.given_cell_chip(15).await;
    world.then_cell_network_configs_are(15, &[]).await;

    let slirp_quirks = Quirks { dynamic_slirp_lease: true, ..Default::default() };
    world.given_cell_chip_with_quirks(20, slirp_quirks, custom_config.clone()).await;
    world.then_cell_network_configs_are(20, &custom_config).await;

    // Then the first SLIRP IPv4 lease remains unallocated and available for chip 30
    world.then_slirp_allocates_ipv4_lease(30, SLIRP_FIRST_V4).await;
}
