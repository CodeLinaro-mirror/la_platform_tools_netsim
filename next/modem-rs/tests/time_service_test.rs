// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

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
    let set_cmd = format!("AT+CCLK={}", time_str);

    when_at_command_sent(&mut world, "A", &set_cmd);
    then_response_is(&mut world, "A", "OK");

    when_at_command_sent(&mut world, "A", "AT+CCLK?");
    then_response_is(&mut world, "A", &format!("+CCLK: {}", time_str));
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Update Network Time
//   Given a modem "A"
//   When external network time update "25/01/01,12:00:00+04" is received
//   Then response from "A" is "+CTZV: +04"
//   When AT command "AT+CCLK?" is sent to "A"
//   Then response from "A" matches "+CCLK: \"25/01/01,12:00:00+04\""
//   And response from "A" is "OK"
#[test]
fn test_update_network_time() {
    let mut world = World::new();
    given_modem(&mut world, "A");
    let id_a = world.modems.get("A").unwrap().0;

    let time_str = "25/01/01,12:00:00+04";
    world.manager.update_network_time(id_a, time_str);

    // Expect NITZ unsolicited
    then_response_is(&mut world, "A", "+CTZV: +04");

    // Expect CCLK updated
    when_at_command_sent(&mut world, "A", "AT+CCLK?");
    then_response_is(&mut world, "A", &format!("+CCLK: \"{}\"", time_str));
    then_response_is(&mut world, "A", "OK");
}
