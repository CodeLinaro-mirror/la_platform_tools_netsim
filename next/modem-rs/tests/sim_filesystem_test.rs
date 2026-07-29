// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{common::constants::*, steps::*, world::World};

// Scenario: Read ICCID from SIM Filesystem
//   Given a modem "A"
//   When AT command 'AT+CRSM=176,12258,0,0,10' is sent to "A"
//   Then response from "A" is '+CRSM: 144,0,89012345678901234567'
//   And response from "A" is "OK"
#[test]
fn test_read_iccid() {
    let mut world = World::new();
    given_modem_with_sim_profile(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CRSM=176,12258,0,0,10");
    then_response_is(&mut world, "A", &format!("+CRSM: 144,0,{}", TEST_ICCID_SWAPPED));
    then_response_is(&mut world, "A", "OK");
}

// Scenario: Select Master File
//   Given a modem "A"
//   When AT command 'AT+CRSM=164,16128,0,0,0' is sent to "A"
//   Then response from "A" is '+CRSM: 144,0,6210'
//   And response from "A" is "OK"
#[test]
fn test_select_mf() {
    let mut world = World::new();
    given_modem_with_sim_profile(&mut world, "A");
    when_at_command_sent(&mut world, "A", "AT+CRSM=164,16128,0,0,0");
    then_response_is(&mut world, "A", "+CRSM: 144,0,6210");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_sim_io_file_not_found() {
    let mut world = World::new();
    given_modem_with_sim_profile(&mut world, "A");
    // Query a dummy file ID 9999 (0x270F) which doesn't exist
    when_at_command_sent(&mut world, "A", "AT+CRSM=176,9999,0,0,10");
    then_response_is(&mut world, "A", "+CRSM: 106,130");
    then_response_is(&mut world, "A", "OK");
}

#[test]
fn test_read_iccid_from_xml() {
    let mut world = World::new();
    let xml = r#"<IccProfile>
    <MF path="3F00">
        <EF name="EF_ICCID" id="2FE2" structure="transparent">
            <SIMIO cmd="B0" p1="0" p2="0" p3="A" data="">144,0,89860318640220133897</SIMIO>
            <CCID>89860318640220133897</CCID>
        </EF>
    </MF>
</IccProfile>"#;
    given_modem_with_xml_profile(&mut world, "A", xml);
    when_at_command_sent(&mut world, "A", "AT+CRSM=176,12258,0,0,10");
    then_response_is(&mut world, "A", "+CRSM: 144,0,98683081462002318379");
    then_response_is(&mut world, "A", "OK");
}
