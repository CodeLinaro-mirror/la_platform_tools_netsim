// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::time::Duration;

use modem_rs::{ModemId, RegistrationStatus};
use netsim_model::{ChipId, ModemAction};

use crate::world::World;

/// Sends an AT command string to the specified modem.
///
/// This function automatically appends `\r\n` if the command string does not
/// already end with it. The command is sent as bytes.
///
/// # Panics
///
/// Panics if the modem name is not found.
pub fn when_at_command_sent(world: &mut World, name: &str, command: &str) {
    let (id, _) = world.get_modem(name);
    let full_command =
        if command.ends_with("\r\n") { command.to_string() } else { format!("{command}\r\n") };

    world.manager.send_at_command(id, full_command.as_bytes());
}

/// Sends raw bytes, specified as a hex string, to the modem.
///
/// This is useful for sending non-ASCII sequences like PDU bytes or Ctrl-Z.
///
/// # Panics
///
/// Panics if the hex string is invalid or the modem is not found.
pub fn when_hex_bytes_sent(world: &mut World, name: &str, hex_bytes: &str) {
    let (id, _) = world.get_modem(name);
    let bytes = hex::decode(hex_bytes).expect("Invalid hex string provided to when_hex_bytes_sent");

    world.manager.send_at_command(id, &bytes);
}

/// Advances the simulated time by `ms` milliseconds and ticks the manager.
///
/// This ensures any time-dependent events (like timeouts) are processed.
pub fn when_time_advances_ms(world: &mut World, ms: u64) {
    world.clock.advance(Duration::from_millis(ms));
    // Ticking allows the manager to process expired timers immediately.
    world.manager.tick();
}

/// Triggers a physical channel configuration update on the modem.
///
/// This invokes the `on_update_physical_channel_configs` method on the modem's
/// data service, simulating network-side changes.
pub fn when_physical_channel_configs_updated(world: &mut World, name: &str) {
    let (id, _) = world.get_modem(name);
    world.manager.update_physical_channel_configs(id);
}

pub fn when_incoming_sms_received(world: &mut World, id: ModemId, sender: &str, text: &str) {
    world.manager.send_incoming_sms(id, sender, text);
}

pub fn when_incoming_pdu_received(world: &mut World, id: ModemId, pdu: &str) {
    world.manager.send_incoming_pdu(id, pdu);
}

pub fn when_external_call_initiated(world: &mut World, target_id: ModemId, number: &str) {
    world.manager.initiate_external_incoming_call(target_id, number);
}

pub fn when_external_call_answered(world: &mut World, id: ModemId) {
    world.manager.initiate_external_answer(id);
}

pub fn when_external_call_hungup(world: &mut World, id: ModemId) {
    world.manager.initiate_external_hangup(id);
}

pub fn when_external_call_held(world: &mut World, id: ModemId, on_hold: bool) {
    world.manager.set_call_hold(id, on_hold);
}

pub fn when_network_time_updated(world: &mut World, id: ModemId, time: &str) {
    world.manager.update_network_time(id, time);
}

pub fn when_voice_registration_set(world: &mut World, id: ModemId, status: RegistrationStatus) {
    world.manager.set_voice_registration(id, status);
}

pub fn when_data_registration_set(world: &mut World, id: ModemId, status: RegistrationStatus) {
    world.manager.set_data_registration(id, status);
}

pub fn when_signal_strength_set(world: &mut World, id: ModemId, rssi: u8, ber: u8) {
    world.manager.set_signal_strength(id, rssi, ber);
}

pub fn when_action_incoming_call(world: &mut World, target_name: &str, number: &str) {
    let (id, _) = world.get_modem(target_name);
    let action = ModemAction::IncomingCall { target_id: ChipId(id), number: number.to_string() };
    world.manager.dispatch(action);
}

pub fn when_action_incoming_sms(world: &mut World, id_name: &str, sender: &str, text: &str) {
    let (id, _) = world.get_modem(id_name);
    let action = ModemAction::IncomingSms {
        id: ChipId(id),
        sender: sender.to_string(),
        text: text.to_string(),
    };
    world.manager.dispatch(action);
}

pub fn when_action_set_voice_registration(
    world: &mut World,
    id_name: &str,
    status: RegistrationStatus,
) {
    let (id, _) = world.get_modem(id_name);
    let action = ModemAction::SetVoiceRegistration { id: ChipId(id), status };
    world.manager.dispatch(action);
}
