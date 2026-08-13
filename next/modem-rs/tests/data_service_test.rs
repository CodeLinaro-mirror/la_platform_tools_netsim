// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{common::constants::*, steps::*, world::World};

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
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGDCONT={TEST_PDP_CID},\"{TEST_PDP_TYPE}\",\"{TEST_APN}\""),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGEQMIN={TEST_PDP_CID},{TEST_QOS_PARAMS_3G}"),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CGEQMIN?");
    then_response_is(&mut world, "A", &format!("+CGEQMIN: {TEST_PDP_CID},{TEST_QOS_PARAMS_3G}"));
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

    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGDCONT={TEST_PDP_CID},\"{TEST_PDP_TYPE}\",\"{TEST_APN}\""),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", &format!("AT+CGACT=1,{TEST_PDP_CID}"));
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
    then_response_is(&mut world, "A", &format!("+CGEV: NW PDN DEACT {TEST_PDP_CID}"));
}

// Scenario: Read Dynamic Parameters
//   Given a modem "A"
//   When AT command 'AT+CGDCONT=1,"IP","test"' is sent to "A"
//   And AT command "ATD*99***1#" is sent to "A"
//   And AT command "AT+CGCONTRDP=1" is sent to "A"
//   Then response from "A" is '+CGCONTRDP:
// 1,5,"test","10.0.2.15/24","10.0.2.2","10.0.2.3"'   And response from "A" is
// "OK"
#[test]
fn test_read_dynamic_param() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGDCONT={TEST_PDP_CID},\"{TEST_PDP_TYPE}\",\"{TEST_APN}\""),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", &format!("ATD{GPRS_DIAL_STRING_PREFIX}{TEST_PDP_CID}#"));
    then_response_is(&mut world, "A", "CONNECT");

    when_at_command_sent(&mut world, "A", &format!("AT+CGCONTRDP={TEST_PDP_CID}"));
    then_response_is(
        &mut world,
        "A",
        &format!(
            "+CGCONTRDP: {TEST_PDP_CID},{DEFAULT_BEARER_ID},\"{TEST_APN}\",{TEST_IP_ADDR},{TEST_GATEWAY},{TEST_DNS}"
        ),
    );
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

    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGDCONT={TEST_PDP_CID},\"{TEST_PDP_TYPE}\",\"{TEST_APN}\""),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGEQREQ={TEST_PDP_CID},{TEST_QOS_PARAMS_3G}"),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CGEQREQ?");
    then_response_is(&mut world, "A", &format!("+CGEQREQ: {TEST_PDP_CID},{TEST_QOS_PARAMS_3G}"));
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

    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGDCONT={TEST_PDP_CID},\"{TEST_PDP_TYPE}\",\"{TEST_APN}\""),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGQMIN={TEST_PDP_CID},{TEST_QOS_PARAMS_3G}"),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CGQMIN?");
    then_response_is(&mut world, "A", &format!("+CGQMIN: {TEST_PDP_CID},{TEST_QOS_PARAMS_3G}"));
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

    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGDCONT={TEST_PDP_CID},\"{TEST_PDP_TYPE}\",\"{TEST_APN}\""),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGQREQ={TEST_PDP_CID},{TEST_QOS_PARAMS_3G}"),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CGQREQ?");
    then_response_is(&mut world, "A", &format!("+CGQREQ: {TEST_PDP_CID},{TEST_QOS_PARAMS_3G}"));
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
//   When AT command 'AT+CGDCONT=1,"IP","test"' is sent to "A"
//   And AT command "AT+CGCMOD=1" is sent to "A"
//   Then response from "A" is "OK"
#[test]
fn test_set_pdp_context_modify() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGDCONT={TEST_PDP_CID},\"{TEST_PDP_TYPE}\",\"{TEST_APN}\""),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", &format!("AT+CGCMOD={TEST_PDP_CID}"));
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Enter Data State
//   Given a modem "A"
//   When AT command 'AT+CGDCONT=1,"IP","test"' is sent to "A"
//   And AT command "AT+CGDATA=1" is sent to "A"
//   Then response from "A" is "CONNECT"
#[test]
fn test_enter_data_state() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGDCONT={TEST_PDP_CID},\"{TEST_PDP_TYPE}\",\"{TEST_APN}\""),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", &format!("AT+CGDATA={TEST_PDP_CID}"));
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
//   Then response from "A" is '+CGPADDR: 1,"10.0.2.15"'
//   And response from "A" is "OK"
#[test]
fn test_show_pdp_address() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // 1. Define context (auto-activated by default)
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGDCONT={TEST_PDP_CID},\"{TEST_PDP_TYPE}\",\"{TEST_APN}\""),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");

    // 2. Query address (should be active IP)
    when_at_command_sent(&mut world, "A", &format!("AT+CGPADDR={TEST_PDP_CID}"));
    then_response_is(&mut world, "A", &format!("+CGPADDR: {TEST_PDP_CID},\"{TEST_IP_ADDR_ONLY}\""));
    then_response_is(&mut world, "A", "OK");

    // 3. Deactivate context (using Goldfish-style AT+CGACT=cid,state instead of
    //    standard state,cid)
    when_at_command_sent(&mut world, "A", &format!("AT+CGACT={TEST_PDP_CID},0"));
    then_response_is(&mut world, "A", "OK");

    // 4. Query address (should be ERROR because inactive)
    when_at_command_sent(&mut world, "A", &format!("AT+CGPADDR={TEST_PDP_CID}"));
    then_response_is(&mut world, "A", "ERROR");

    // 5. Reactivate context (Goldfish-style AT+CGACT=cid,state)
    when_at_command_sent(&mut world, "A", &format!("AT+CGACT={TEST_PDP_CID},1"));
    then_response_is(&mut world, "A", "OK");

    // 6. Query address (should be active IP again)
    when_at_command_sent(&mut world, "A", &format!("AT+CGPADDR={TEST_PDP_CID}"));
    then_response_is(&mut world, "A", &format!("+CGPADDR: {TEST_PDP_CID},\"{TEST_IP_ADDR_ONLY}\""));
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_gprs_dialing_fallback() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // 1. Define the context first
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGDCONT={TEST_PDP_CID},\"{TEST_PDP_TYPE}\",\"{TEST_APN}\""),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");

    // 2. Dial GPRS fallback which should return CONNECT
    when_at_command_sent(&mut world, "A", &format!("ATD{GPRS_DIAL_STRING_PREFIX}{TEST_PDP_CID}#"));
    then_response_is(&mut world, "A", "CONNECT");

    // 3. Verify that the PDP context is indeed active now (returns active IP)
    when_at_command_sent(&mut world, "A", &format!("AT+CGPADDR={TEST_PDP_CID}"));
    then_response_is(&mut world, "A", &format!("+CGPADDR: {TEST_PDP_CID},\"{TEST_IP_ADDR_ONLY}\""));
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_dial_non_existent_context() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Dial GPRS context 2 which is undefined -> should return ERROR
    when_at_command_sent(
        &mut world,
        "A",
        &format!("ATD{GPRS_DIAL_STRING_PREFIX}{TEST_PDP_CID_ALT}#"),
    );
    then_response_is(&mut world, "A", "ERROR");
}

#[test]
fn test_gprs_dialing_default_cid() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // Define PDP context 1
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGDCONT={TEST_PDP_CID},\"{TEST_PDP_TYPE}\",\"{TEST_APN}\""),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");

    // Dialing *99# without specified CID should default to PDP context 1
    when_at_command_sent(&mut world, "A", "ATD*99#");
    then_response_is(&mut world, "A", "CONNECT");

    // Verify PDP context 1 is indeed active
    when_at_command_sent(&mut world, "A", &format!("AT+CGPADDR={TEST_PDP_CID}"));
    then_response_is(&mut world, "A", &format!("+CGPADDR: {TEST_PDP_CID},\"{TEST_IP_ADDR_ONLY}\""));
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_multiple_concurrent_pdp_contexts() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // 1. Define and activate PDP context 1
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGDCONT={TEST_PDP_CID},\"{TEST_PDP_TYPE}\",\"{TEST_APN1}\""),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", &format!("AT+CGACT=1,{TEST_PDP_CID}"));
    then_wait_for_response_containing(&mut world, "A", "OK");

    // 2. Define and activate PDP context 2
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGDCONT={TEST_PDP_CID_ALT},\"{TEST_PDP_TYPE}\",\"{TEST_APN2}\""),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", &format!("AT+CGACT=1,{TEST_PDP_CID_ALT}")); // Activate CID 2
    then_wait_for_response_containing(&mut world, "A", "OK");

    // 3. Verify unique IP addresses via AT+CGPADDR
    when_at_command_sent(&mut world, "A", &format!("AT+CGPADDR={TEST_PDP_CID}"));
    then_response_is(&mut world, "A", &format!("+CGPADDR: {TEST_PDP_CID},\"{TEST_IP_ADDR_ONLY}\""));
    then_response_is(&mut world, "A", "OK");

    // Verify unique IP addresses via AT+CGPADDR
    when_at_command_sent(&mut world, "A", &format!("AT+CGPADDR={TEST_PDP_CID_ALT}"));
    then_response_is(
        &mut world,
        "A",
        &format!("+CGPADDR: {TEST_PDP_CID_ALT},\"{TEST_IP_ADDR_ALT_ONLY}\""),
    );
    then_response_is(&mut world, "A", "OK");

    // 4. Verify unique IP addresses via AT+CGCONTRDP
    when_at_command_sent(&mut world, "A", &format!("AT+CGCONTRDP={TEST_PDP_CID}"));
    then_response_is(
        &mut world,
        "A",
        &format!(
            "+CGCONTRDP: {TEST_PDP_CID},{DEFAULT_BEARER_ID},\"{TEST_APN1}\",{TEST_IP_ADDR},{TEST_GATEWAY},{TEST_DNS}"
        ),
    );
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", &format!("AT+CGCONTRDP={TEST_PDP_CID_ALT}"));
    then_response_is(
        &mut world,
        "A",
        &format!(
            "+CGCONTRDP: {TEST_PDP_CID_ALT},{DEFAULT_BEARER_ID},\"{TEST_APN2}\",{TEST_IP_ADDR_ALT},{TEST_GATEWAY},{TEST_DNS}"
        ),
    );
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_goldfish_ril_compat_incorrect_cgact() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // 1. Define and activate PDP context 1
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGDCONT={TEST_PDP_CID},\"{TEST_PDP_TYPE}\",\"{TEST_APN}\""),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");
    when_at_command_sent(&mut world, "A", &format!("AT+CGACT=1,{TEST_PDP_CID}"));
    then_wait_for_response_containing(&mut world, "A", "OK");

    // Verify it is active
    when_at_command_sent(&mut world, "A", &format!("AT+CGPADDR={TEST_PDP_CID}"));
    then_response_is(&mut world, "A", &format!("+CGPADDR: {TEST_PDP_CID},\"{TEST_IP_ADDR_ONLY}\""));
    then_response_is(&mut world, "A", "OK");

    // 2. Send the goldfish RIL "deactivate CID 1" command: AT+CGACT=1,0
    // (Standard would be AT+CGACT=0,1)
    when_at_command_sent(&mut world, "A", &format!("AT+CGACT={TEST_PDP_CID},0"));
    then_wait_for_response_containing(&mut world, "A", "OK");

    // 3. Verify it is now inactive (should return ERROR)
    when_at_command_sent(&mut world, "A", &format!("AT+CGPADDR={TEST_PDP_CID}"));
    then_response_is(&mut world, "A", "ERROR");

    // 4. Define CID 2
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGDCONT={TEST_PDP_CID_ALT},\"{TEST_PDP_TYPE}\",\"test2\""),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");

    // 5. Activate CID 2 using legacy format: AT+CGACT=2,1 (Standard is
    //    AT+CGACT=1,2)
    when_at_command_sent(&mut world, "A", &format!("AT+CGACT={TEST_PDP_CID_ALT},1"));
    then_wait_for_response_containing(&mut world, "A", "OK");

    // Verify CID 2 is active
    when_at_command_sent(&mut world, "A", &format!("AT+CGPADDR={TEST_PDP_CID_ALT}"));
    then_response_is(
        &mut world,
        "A",
        &format!("+CGPADDR: {TEST_PDP_CID_ALT},\"{TEST_IP_ADDR_ALT_ONLY}\""),
    );
    then_response_is(&mut world, "A", "OK");

    // 6. Deactivate CID 2 using legacy format: AT+CGACT=2,0 (Standard is
    //    AT+CGACT=0,2)
    when_at_command_sent(&mut world, "A", &format!("AT+CGACT={TEST_PDP_CID_ALT},0"));
    then_wait_for_response_containing(&mut world, "A", "OK");

    // Verify CID 2 is inactive (should return ERROR)
    when_at_command_sent(&mut world, "A", &format!("AT+CGPADDR={TEST_PDP_CID_ALT}"));
    then_response_is(&mut world, "A", "ERROR");
}

#[test]
fn test_query_pdp_context_activate() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // 1. Define context 1 and 2
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGDCONT={TEST_PDP_CID},\"{TEST_PDP_TYPE}\",\"{TEST_APN1}\""),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGDCONT={TEST_PDP_CID_ALT},\"{TEST_PDP_TYPE}\",\"{TEST_APN2}\""),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");

    // By default, contexts are auto-activated in Goldfish mode.
    when_at_command_sent(&mut world, "A", "AT+CGACT?");
    then_response_is(&mut world, "A", &format!("+CGACT: {TEST_PDP_CID},1"));
    then_response_is(&mut world, "A", &format!("+CGACT: {TEST_PDP_CID_ALT},1"));
    then_response_is(&mut world, "A", "OK");

    // Deactivate context 1
    when_at_command_sent(&mut world, "A", &format!("AT+CGACT={TEST_PDP_CID},0"));
    then_wait_for_response_containing(&mut world, "A", "OK");

    // Query again
    when_at_command_sent(&mut world, "A", "AT+CGACT?");
    then_response_is(&mut world, "A", &format!("+CGACT: {TEST_PDP_CID},0"));
    then_response_is(&mut world, "A", &format!("+CGACT: {TEST_PDP_CID_ALT},1"));
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_ps_attach_detach() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // 1. Verify initially attached
    when_at_command_sent(&mut world, "A", "AT+CGATT?");
    then_response_is(&mut world, "A", "+CGATT: 1");
    then_response_is(&mut world, "A", "OK");

    // 2. Define a context
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGDCONT={TEST_PDP_CID},\"{TEST_PDP_TYPE}\",\"{TEST_APN}\""),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");

    // 3. Detach PS
    when_at_command_sent(&mut world, "A", "AT+CGATT=0");
    then_wait_for_response_containing(&mut world, "A", "OK");

    // 4. Verify detached
    when_at_command_sent(&mut world, "A", "AT+CGATT?");
    then_response_is(&mut world, "A", "+CGATT: 0");
    then_response_is(&mut world, "A", "OK");

    // 5. Verify context is deactivated
    when_at_command_sent(&mut world, "A", "AT+CGACT?");
    then_response_is(&mut world, "A", &format!("+CGACT: {TEST_PDP_CID},0"));
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_qos_queries_when_empty() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // No contexts defined, queries should return OK (no data lines)
    when_at_command_sent(&mut world, "A", "AT+CGEQMIN?");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CGEQREQ?");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CGQMIN?");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CGQREQ?");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_data_commands_invalid_cid() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    // CID 2 does not exist (no contexts defined at all, or we only define CID 1)
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGDCONT={TEST_PDP_CID},\"{TEST_PDP_TYPE}\",\"{TEST_APN}\""),
    );
    then_wait_for_response_containing(&mut world, "A", "OK");

    // 1. QoS settings on non-existent CID 2
    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGEQMIN={INVALID_PDP_CID},{TEST_QOS_PARAMS_GPRS}"),
    );
    then_response_is(&mut world, "A", "ERROR");

    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGEQREQ={INVALID_PDP_CID},{TEST_QOS_PARAMS_GPRS}"),
    );
    then_response_is(&mut world, "A", "ERROR");

    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGQMIN={INVALID_PDP_CID},{TEST_QOS_PARAMS_GPRS}"),
    );
    then_response_is(&mut world, "A", "ERROR");

    when_at_command_sent(
        &mut world,
        "A",
        &format!("AT+CGQREQ={INVALID_PDP_CID},{TEST_QOS_PARAMS_GPRS}"),
    );
    then_response_is(&mut world, "A", "ERROR");

    // 2. Data commands on non-existent CID 2
    when_at_command_sent(&mut world, "A", &format!("AT+CGACT=1,{INVALID_PDP_CID}")); // Activate CID 2 (state=1, cid=2)
    then_response_is(&mut world, "A", "ERROR");

    when_at_command_sent(&mut world, "A", &format!("AT+CGCMOD={INVALID_PDP_CID}")); // Modify CID 2
    then_response_is(&mut world, "A", "ERROR");

    when_at_command_sent(&mut world, "A", &format!("AT+CGPADDR={INVALID_PDP_CID}")); // Show address for CID 2
    then_response_is(&mut world, "A", "ERROR");

    when_at_command_sent(&mut world, "A", &format!("AT+CGCONTRDP={INVALID_PDP_CID}")); // Read dynamic param for CID 2
    then_response_is(&mut world, "A", "ERROR");

    when_at_command_sent(&mut world, "A", &format!("AT+CGDATA={INVALID_PDP_CID}")); // Enter data state for CID 2
    then_response_is(&mut world, "A", "ERROR");
}
