// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::world::World;

#[test]
fn test_goldfish_boot_sequence() {
    let mut world = World::new();
    world.given_modem_with_sim_profile("A");

    // The exact initialization sequence from Goldfish HALS main.cpp:

    // 1. ATE0Q0V1
    world.send_and_expect_ok("A", "ATE0Q0V1");

    // 2. AT+CMEE=1
    world.send_and_expect_ok("A", "AT+CMEE=1");

    // 3. AT+CREG=2
    world.send_and_expect_ok("A", "AT+CREG=2");

    // 4. AT+CGREG=2
    world.send_and_expect_ok("A", "AT+CGREG=2");

    // 5. AT+CEREG=2
    world.send_and_expect_ok("A", "AT+CEREG=2");

    // 6. AT+CCWA=1
    world.send_and_expect_ok("A", "AT+CCWA=1");

    // 7. AT+CMOD=0
    world.send_and_expect_ok("A", "AT+CMOD=0");

    // 8. AT+CMUT=0
    world.send_and_expect_ok("A", "AT+CMUT=0");

    // 9. AT+CSSN=0,1
    world.send_and_expect_ok("A", "AT+CSSN=0,1");

    // 10. AT+COLP=0
    world.send_and_expect_ok("A", "AT+COLP=0");

    // 11. AT+CSCS="HEX"
    world.send_and_expect_ok("A", "AT+CSCS=\"HEX\"");

    // 12. AT+CUSD=1
    world.send_and_expect_ok("A", "AT+CUSD=1");

    // 13. AT+CGEREP=1,0
    world.send_and_expect_ok("A", "AT+CGEREP=1,0");

    // 14. AT+CMGF=0
    world.send_and_expect_ok("A", "AT+CMGF=0");

    // 15. AT+CFUN?
    world.send_and_expect("A", "AT+CFUN?", &["+CFUN: 1", "OK"]);
}
