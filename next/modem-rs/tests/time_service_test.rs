// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#[cfg(not(target_os = "windows"))]
use jiff::Zoned;

#[cfg(not(target_os = "windows"))]
use crate::common::constants::RESP_CSQ_LTE_DEFAULT;
use crate::{steps::*, world::World};

// Scenario: Set and Query Time
//   Given a modem "A"
//   When AT command 'AT+CCLK="25/08/02,12:30:00+00"' is sent to "A"
//   Then response from "A" is "OK"
//   When AT command "AT+CCLK?" is sent to "A"
//   Then response from "A" is '+CCLK: "25/08/02,12:30:00+00"'
//   And response from "A" is "OK"
#[test]
fn test_set_and_query_time() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    let time_str = "\"25/08/02,12:30:00+00\"";
    let set_cmd = format!("AT+CCLK={time_str}");

    when_at_command_sent(&mut world, "A", &set_cmd);
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CCLK?");
    then_response_is(&mut world, "A", &format!("+CCLK: {time_str}"));
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Update Network Time
//   Given a modem "A"
//   When external network time update is received
//   Then response from "A" is "%CTZV: ..."
//   When AT command "AT+CCLK?" is sent to "A"
//   Then response from "A" matches "+CCLK: ..."
//   And response from "A" is "OK"
#[test]
#[cfg(not(target_os = "windows"))]
fn test_update_network_time() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    let zoned: Zoned = "2026-06-15T12:30:45-07:00[America/Los_Angeles]".parse().unwrap();
    world.clock.set_zoned(zoned);

    when_network_time_updated(&mut world, "A");

    // Expect NITZ unsolicited (%CTZV: yy/MM/dd:HH:mm:ss+/-tz:dst:zone_name)
    then_response_is(&mut world, "A", "%CTZV: 26/06/15:19:30:45-28:1:America!Los_Angeles");

    // Expect CCLK queries current time from clock
    when_at_command_sent(&mut world, "A", "AT+CCLK?");
    then_response_is(&mut world, "A", "+CCLK: \"26/06/15,19:30:45-28\"");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_ctzv_command() {
    let mut world = World::new();
    given_modem(&mut world, "A");

    when_at_command_sent(&mut world, "A", "AT%CTZV?");
    then_response_is(&mut world, "A", "%CTZV: 0");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT%CTZV=1");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT%CTZV?");
    then_response_is(&mut world, "A", "%CTZV: 1");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT%CTZV=0");
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT%CTZV?");
    then_response_is(&mut world, "A", "%CTZV: 0");
    then_response_is(&mut world, "A", "OK");
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_auto_ctzv_on_attach() {
    let mut world = World::new();
    let zoned: Zoned = "2026-06-15T12:30:45-07:00[America/Los_Angeles]".parse().unwrap();
    world.clock.set_zoned(zoned);

    // Modems with auto_ctzv enabled (e.g. Goldfish and Cuttlefish) emit %CTZV on
    // network attach
    given_modem_with_quirks(
        &mut world,
        "A",
        netsim_model::Quirks { auto_ctzv: true, ..Default::default() },
    );

    // By default, AT%CTZV? should report enabled (1)
    when_at_command_sent(&mut world, "A", "AT%CTZV?");
    then_response_is(&mut world, "A", "%CTZV: 1");
    then_response_is(&mut world, "A", "OK");

    // Turn radio ON to schedule network attachment
    when_at_command_sent(&mut world, "A", "AT+CFUN=1");
    then_response_is(&mut world, "A", "OK");

    // Advance time to trigger the AttachNetwork event (10ms delay)
    when_time_advances_ms(&mut world, 15);

    // Verify unsolicited %CTZV NITZ update is received
    then_wait_for_response_containing(
        &mut world,
        "A",
        "%CTZV: 26/06/15:19:30:45-28:1:America!Los_Angeles",
    );
}

#[test]
#[cfg(not(target_os = "windows"))]
fn test_auto_ctzv_disabled_by_default() {
    let mut world = World::new();
    let zoned: Zoned = "2026-06-15T12:30:45-07:00[America/Los_Angeles]".parse().unwrap();
    world.clock.set_zoned(zoned);

    // Default modem (auto_ctzv: false) should NOT emit %CTZV on network attach
    given_modem(&mut world, "A");

    // By default, AT%CTZV? should report disabled (0)
    when_at_command_sent(&mut world, "A", "AT%CTZV?");
    then_response_is(&mut world, "A", "%CTZV: 0");
    then_response_is(&mut world, "A", "OK");

    // Turn radio ON to schedule network attachment
    when_at_command_sent(&mut world, "A", "AT+CFUN=1");
    then_response_is(&mut world, "A", "OK");

    // Advance time to trigger AttachNetwork event
    when_time_advances_ms(&mut world, 15);

    // In a default modem, only standard CSQ is sent on attach, no %CTZV response
    then_response_is(&mut world, "A", RESP_CSQ_LTE_DEFAULT);
    then_no_response(&mut world, "A");
}
