// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{steps::*, world::World};

#[test]
fn test_goldfish_boot_sequence() {
    let mut world = World::new();
    given_modem_with_sim_profile(&mut world, "A");

    // The exact initialization sequence from Goldfish HALS main.cpp:

    // 1. ATE0Q0V1
    when_at_command_sent(&mut world, "A", "ATE0Q0V1");
    then_response_is(&mut world, "A", "OK");

    // 2. AT+CMEE=1
    when_at_command_sent(&mut world, "A", "AT+CMEE=1");
    then_response_is(&mut world, "A", "OK");

    // 3. AT+CREG=2
    when_at_command_sent(&mut world, "A", "AT+CREG=2");
    then_response_is(&mut world, "A", "OK");

    // 4. AT+CGREG=2
    when_at_command_sent(&mut world, "A", "AT+CGREG=2");
    then_response_is(&mut world, "A", "OK");

    // 5. AT+CEREG=2
    when_at_command_sent(&mut world, "A", "AT+CEREG=2");
    then_response_is(&mut world, "A", "OK");

    // 6. AT+CCWA=1
    when_at_command_sent(&mut world, "A", "AT+CCWA=1");
    then_response_is(&mut world, "A", "OK");

    // 7. AT+CMOD=0
    when_at_command_sent(&mut world, "A", "AT+CMOD=0");
    then_response_is(&mut world, "A", "OK");

    // 8. AT+CMUT=0
    when_at_command_sent(&mut world, "A", "AT+CMUT=0");
    then_response_is(&mut world, "A", "OK");

    // 9. AT+CSSN=0,1
    when_at_command_sent(&mut world, "A", "AT+CSSN=0,1");
    then_response_is(&mut world, "A", "OK");

    // 10. AT+COLP=0
    when_at_command_sent(&mut world, "A", "AT+COLP=0");
    then_response_is(&mut world, "A", "OK");

    // 11. AT+CSCS="HEX"
    when_at_command_sent(&mut world, "A", "AT+CSCS=\"HEX\"");
    then_response_is(&mut world, "A", "OK");

    // 12. AT+CUSD=1
    when_at_command_sent(&mut world, "A", "AT+CUSD=1");
    then_response_is(&mut world, "A", "OK");

    // 13. AT+CGEREP=1,0
    when_at_command_sent(&mut world, "A", "AT+CGEREP=1,0");
    then_response_is(&mut world, "A", "OK");

    // 14. AT+CMGF=0
    when_at_command_sent(&mut world, "A", "AT+CMGF=0");
    then_response_is(&mut world, "A", "OK");

    // 15. AT+CFUN?
    when_at_command_sent(&mut world, "A", "AT+CFUN?");
    then_response_is(&mut world, "A", "+CFUN: 1");
    then_response_is(&mut world, "A", "OK");
}
