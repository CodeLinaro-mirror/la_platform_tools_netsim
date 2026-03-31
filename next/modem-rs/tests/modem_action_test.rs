// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_model::cell::RegistrationStatus;

use crate::{steps::*, world::World};

// Scenario: Trigger Incoming Call via Action
//   Given a modem "A"
//   When Action IncomingCall is dispatched to "A" from "123456"
//   Then response from "A" is "RING"
//   And response from "A" is "+CLIP: \"123456\",129,,,,0"
#[test]
fn test_action_incoming_call() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_action_incoming_call(&mut world, "A", "123456");

    then_response_is(&mut world, "A", "RING");
    then_response_is(&mut world, "A", "+CLIP: \"123456\",129,,,,0");
}

// Scenario: Trigger Incoming SMS via Action
//   Given a modem "A"
//   When Action IncomingSms is dispatched to "A" from "5555" with text "Hello"
//   Then response from "A" is contains "+CMT:"
//   And response from "A" is "Hello"
#[test]
fn test_action_incoming_sms() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_action_incoming_sms(&mut world, "A", "5555", "Hello");

    // Check for CMT line and body in the same response
    let response = then_wait_for_response_containing(&mut world, "A", "+CMT: \"5555\"");
    assert!(response.contains("Hello"));
}

// Scenario: Set Registration Status via Action
//   Given a modem "A"
//   When Action SetVoiceRegistration is dispatched to "A" with status Roaming
//   Then response from "A" is "+CREG: 5"
#[test]
fn test_action_set_registration() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_action_set_voice_registration(&mut world, "A", RegistrationStatus::Roaming);

    then_response_is(&mut world, "A", "+CREG: 5");
}
