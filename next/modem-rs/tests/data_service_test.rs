// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{steps::*, world::World};

// Scenario: Set and Query QoS Minimum
//   Given a modem "A"
//   When AT command 'AT+CGDCONT=1,"IP","test"' is sent to "A"
//   And AT command "AT+CGEQMIN=1,2,3,4,5,6" is sent to "A"
//   And AT command "AT+CGEQMIN?" is sent to "A"
//   Then response from "A" is "+CGEQMIN: 1,2,3,4,5,6"
//   And response from "A" is "OK"
#[test]
fn test_set_and_query_quality_of_service_minimum() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Setup context
    when_at_command_sent(&mut world, "A", "AT+CGDCONT=1,\"IP\",\"test\"");
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CGEQMIN=1,2,3,4,5,6");
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CGEQMIN?");
    then_response_is(&mut world, "A", "+CGEQMIN: 1,2,3,4,5,6");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Activate PDP Context
//   Given a modem "A"
//   When AT command 'AT+CGDCONT=1,"IP","test"' is sent to "A"
//   And AT command "AT+CGACT=1,1" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_set_pdp_context_activate() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CGDCONT=1,\"IP\",\"test\"");
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CGACT=1,1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Update Physical Channel Configs
//   Given a modem "A"
//   When physical channel configs are updated on "A"
//   Then response from "A" is "+CGEV: NW PDN DEACT 1"
#[test]
fn test_update_physical_channel_configs() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_physical_channel_configs_updated(&mut world, "A");
    then_response_is(&mut world, "A", "+CGEV: NW PDN DEACT 1");
}

// Scenario: Read Dynamic Parameters
//   Given a modem "A"
//   When AT command "AT+CGSCONTRDP=1" is sent to "A"
//   Then response from "A" is "+CGSCONTRDP: 1, 5, 1500, 300000, 300000, 300000,
// 300000"   And response from "A" is "OK"
#[test]
fn test_read_dynamic_param() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CGSCONTRDP=1");
    then_response_is(&mut world, "A", "+CGSCONTRDP: 1, 5, 1500, 300000, 300000, 300000, 300000");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Query PDP Context
//   Given a modem "A"
//   When AT command "AT+CGDCONT?" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_query_pdp_context() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CGDCONT?");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Set and Query QoS Requested
//   Given a modem "A"
//   When AT command 'AT+CGDCONT=1,"IP","test"' is sent to "A"
//   And AT command "AT+CGEQREQ=1,2,3,4,5,6" is sent to "A"
//   And AT command "AT+CGEQREQ?" is sent to "A"
//   Then response from "A" is "+CGEQREQ: 1,2,3,4,5,6"
//   And response from "A" is "OK"
#[test]
fn test_set_and_query_quality_of_service_requested() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CGDCONT=1,\"IP\",\"test\"");
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CGEQREQ=1,2,3,4,5,6");
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CGEQREQ?");
    then_response_is(&mut world, "A", "+CGEQREQ: 1,2,3,4,5,6");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Set and Query QoS Minimum GPRS
//   Given a modem "A"
//   When AT command 'AT+CGDCONT=1,"IP","test"' is sent to "A"
//   And AT command "AT+CGQMIN=1,2,3,4,5,6" is sent to "A"
//   And AT command "AT+CGQMIN?" is sent to "A"
//   Then response from "A" is "+CGQMIN: 1,2,3,4,5,6"
//   And response from "A" is "OK"
#[test]
fn test_set_and_query_quality_of_service_minimum_gprs() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CGDCONT=1,\"IP\",\"test\"");
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CGQMIN=1,2,3,4,5,6");
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CGQMIN?");
    then_response_is(&mut world, "A", "+CGQMIN: 1,2,3,4,5,6");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Set and Query QoS Requested GPRS
//   Given a modem "A"
//   When AT command 'AT+CGDCONT=1,"IP","test"' is sent to "A"
//   And AT command "AT+CGQREQ=1,2,3,4,5,6" is sent to "A"
//   And AT command "AT+CGQREQ?" is sent to "A"
//   Then response from "A" is "+CGQREQ: 1,2,3,4,5,6"
//   And response from "A" is "OK"
#[test]
fn test_set_and_query_quality_of_service_requested_gprs() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CGDCONT=1,\"IP\",\"test\"");
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CGQREQ=1,2,3,4,5,6");
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CGQREQ?");
    then_response_is(&mut world, "A", "+CGQREQ: 1,2,3,4,5,6");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Set PS Attach
//   Given a modem "A"
//   When AT command "AT+CGATT=1" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_set_ps_attach() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CGATT=1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Modify PDP Context
//   Given a modem "A"
//   When AT command "AT+CGCMOD=1" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_set_pdp_context_modify() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CGCMOD=1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Enter Data State
//   Given a modem "A"
//   When AT command "AT+CGDATA=1" is sent to "A"
//   Then response from "A" is "CONNECT"
#[test]
fn test_enter_data_state() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CGDATA=1");
    then_response_is(&mut world, "A", "CONNECT");
}

// Scenario: Set Packet Event Reporting
//   Given a modem "A"
//   When AT command "AT+CGEREP=1,1" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_set_packet_event_reporting() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CGEREP=1,1");
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Show PDP Address
//   Given a modem "A"
//   When AT command 'AT+CGDCONT=1,"IP","test"' is sent to "A"
//   And AT command "AT+CGPADDR=1" is sent to "A"
//   Then response from "A" is '+CGPADDR: 1,"0.0.0.0"'
//   And response from "A" is "OK"
#[test]
fn test_show_pdp_address() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT+CGDCONT=1,\"IP\",\"test\"");
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CGPADDR=1");
    then_response_is(&mut world, "A", "+CGPADDR: 1,\"0.0.0.0\"");
    then_response_is(&mut world, "A", "OK");
}
