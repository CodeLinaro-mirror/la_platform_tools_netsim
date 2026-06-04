// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{steps::*, world::World};

// Scenario: Set CLIP (Calling Line Identification Presentation)
//   Given a modem "A"
//   When AT command "AT+CLIP=1" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_set_clip() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CLIP=1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Set Call Waiting
//   Given a modem "A"
//   When AT command "AT+CCWA=1,1" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_set_call_waiting() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CCWA=1,1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Send USSD
//   Given a modem "A"
//   When AT command 'AT+CUSD=1,"*123#"' is sent to "A"
//   Then response from "A" is '+CUSD: 0,"OK",15'
//   And response from "A" is "OK"
#[test]
fn test_send_ussd() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", r#"AT+CUSD=1,"*123#""#);
    then_response_is(&mut world, "A", r#"+CUSD: 0,"OK",15"#);
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Cancel USSD
//   Given a modem "A"
//   When AT command "AT+CUSD=2" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_cancel_ussd() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CUSD=2");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Call Forwarding
//   Given a modem "A"
//   When AT command 'AT+CCFC=1,1,"+1234567890",145,20' is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_call_forwarding() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", r#"AT+CCFC=1,1,"+1234567890",145,20"#);
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Query CLIR
//   Given a modem "A"
//   When AT command "AT+CLIR?" is sent to "A"
//   Then response from "A" is "+CLIR: 0,0"
//   And response from "A" is "OK"
#[test]
fn test_query_clir() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CLIR?");
    then_response_is(&mut world, "A", "+CLIR: 0,0");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Supplementary Service Notification
//   Given a modem "A"
//   When AT command "AT+CSSN=1,1" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_supp_service_notification() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CSSN=1,1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Set Facility Lock
//   Given a modem "A"
//   When AT command 'AT+CLCK="SC",1,"1234"' is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_set_facility_lock() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", r#"AT+CLCK="SC",1,"1234""#);
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_query_facility_lock() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    when_at_command_sent(&mut world, "A", r#"AT+CLCK="FD",2"#);
    then_response_is(&mut world, "A", "+CLCK: 0");
    then_response_is(&mut world, "A", "OK");
}
