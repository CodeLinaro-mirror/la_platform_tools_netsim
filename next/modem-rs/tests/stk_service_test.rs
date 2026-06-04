// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{steps::*, world::World};

// Scenario: STK Display Text
//   Given a modem "A"
//   When AT command 'AT+CUSATE="D1150121810D050448656C6C6F20576F726C64"' is
// sent to "A"   Then response from "A" is '+CUSAT: "9000"'
#[test]
fn test_stk_display_text() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // This is a simplified "Display Text" proactive command envelope.
    when_at_command_sent(&mut world, "A", "AT+CUSATE=\"D1150121810D050448656C6C6F20576F726C64\"");

    then_response_is(&mut world, "A", "+CUSAT: \"9000\"");
}

// Scenario: Send STK Envelope Command
//   Given a modem "A"
//   When AT command 'AT+CUSATE="D3120101"' is sent to "A"
//   Then response from "A" is '+CUSATP: "SubMenu1"'
#[test]
fn test_send_stk_envelope_command() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CUSATE=\"D3120101\"");

    then_response_is(&mut world, "A", "+CUSATP: \"SubMenu1\"");
}

// Scenario: STK Get Input
//   Given a modem "A"
//   When AT command 'AT+CUSATE="D1150123810D0504456E7465722054657874"' is sent
// to "A"   Then response from "A" is '+CUSAT: "9000"'
#[test]
fn test_stk_get_input() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // This is a simplified "Get Input" proactive command envelope.
    when_at_command_sent(&mut world, "A", "AT+CUSATE=\"D1150123810D0504456E7465722054657874\"");

    then_response_is(&mut world, "A", "+CUSAT: \"9000\"");
}

// Scenario: Query STK Ready
//   Given a modem "A"
//   When AT command "AT+CUSATD?" is sent to "A"
//   Then response from "A" is "+CUSATD: 1, 1"
//   And response from "A" is "OK"
#[test]
fn test_query_stk_ready() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CUSATD?");

    then_response_is(&mut world, "A", "+CUSATD: 1, 1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Set STK Mode
//   Given a modem "A"
//   When AT command "AT+STK=1" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_set_stk() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+STK=1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Set STK Enabled
//   Given a modem "A"
//   When AT command "AT+STKEN=1" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_set_stk_enabled() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+STKEN=1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Set STK Unsolicited Result
//   Given a modem "A"
//   When AT command "AT+STKUR=1" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_set_stk_unsolicited_result() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+STKUR=1");
    then_response_is(&mut world, "A", "OK");
}
