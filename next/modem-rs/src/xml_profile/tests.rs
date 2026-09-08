// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::{
    constants::UiccFileId,
    profiles::{PROFILE_CTS_XML, PROFILE_DEFAULT_XML, PROFILE_TEL_ALASKA_XML},
};

#[test]
fn test_parse_default_profile() {
    let profile = parse_xml_profile(PROFILE_DEFAULT_XML);
    assert!(profile.is_ok(), "Failed to parse default profile: {:?}", profile.err());
    let profile = profile.unwrap();
    assert_eq!(profile.imsi, "310260000000000");
    assert_eq!(profile.iccid, "89860318640220133897");
}

#[test]
fn test_parse_cts_profile() {
    let profile = parse_xml_profile(PROFILE_CTS_XML);
    assert!(profile.is_ok(), "Failed to parse CTS profile: {:?}", profile.err());
    let profile = profile.unwrap();
    assert_eq!(profile.imsi, "310260000000000");
    assert_eq!(profile.iccid, "89860318640220133897");
}

#[test]
fn test_parse_tel_alaska_profile() {
    let profile = parse_xml_profile(PROFILE_TEL_ALASKA_XML);
    assert!(profile.is_ok(), "Failed to parse TelAlaska profile: {:?}", profile.err());
    let profile = profile.unwrap();
    assert_eq!(profile.imsi, "311740123456789");
    assert_eq!(profile.iccid, "89860318640220133897");
}

#[test]
fn test_invalid_record_number_fails() {
    let bad_xml = r#"<IccProfile>
        <MF>
            <EF id="6FC7" structure="linear fixed">
                <SIMIO cmd="B2" p1="00" p2="04" p3="26">90,00,01020304</SIMIO>
            </EF>
        </MF>
    </IccProfile>"#;
    let result = parse_xml_profile(bad_xml);
    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        XmlProfileError::InvalidValue {
            file_id: UiccFileId::MailboxDialingNumbers.as_u16(),
            field: "SIMIO p1 (record number)".to_string(),
            value: "0".to_string(),
            expected: "1-indexed record number between 1 and 255".to_string(),
        }
    );

    let overflow_xml = r#"<IccProfile>
        <MF>
            <EF id="6FC7" structure="linear fixed">
                <SIMIO cmd="B2" p1="100" p2="04" p3="26">90,00,01020304</SIMIO>
            </EF>
        </MF>
    </IccProfile>"#;
    let result = parse_xml_profile(overflow_xml);
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), XmlProfileError::Deserialization(_)));
}

#[test]
fn test_parameter_normalization() {
    let xml = r#"<IccProfile>
        <MF>
            <EF id="6FC7" structure="linear fixed">
                <SIMIO cmd="B2" p1="0A" p2="04" p3="01">90,00,01</SIMIO>
            </EF>
        </MF>
    </IccProfile>"#;
    let profile = parse_xml_profile(xml).unwrap();
    let ef = profile.sim_io.file_system.find_ef(UiccFileId::MailboxDialingNumbers).unwrap();
    assert_eq!(ef.record_len, Some(1));
    assert_eq!(ef.data, hex::decode("FFFFFFFFFFFFFFFFFF01").unwrap());
}

#[test]
fn test_deduce_record_len_from_dc_mappings() {
    let xml = r#"<IccProfile>
        <MF>
            <EF id="6FC7" structure="linear fixed">
                <SIMIO cmd="DC" p1="1" p2="04" p3="26">144,0</SIMIO>
                <SIMIO cmd="DC" p1="2" p2="04" p3="26">144,0</SIMIO>
            </EF>
        </MF>
    </IccProfile>"#;
    let profile = parse_xml_profile(xml).unwrap();
    let ef = profile.sim_io.file_system.find_ef(UiccFileId::MailboxDialingNumbers).unwrap();
    let mbdn_len =
        UiccFileId::MailboxDialingNumbers.default_record_len().expect("file id is record based");
    assert_eq!(ef.record_len, Some(mbdn_len));
    assert_eq!(ef.size(), 2 * mbdn_len);
    assert_eq!(ef.data, vec![0xFF; 2 * mbdn_len]);
}

#[test]
fn test_invalid_file_id_fails() {
    let xml = r#"<IccProfile><MF><EF id="XYZ"></EF></MF></IccProfile>"#;
    let result = parse_xml_profile(xml);
    assert!(matches!(result, Err(XmlProfileError::Deserialization(_))));
}

#[test]
fn test_invalid_hex_param_fails() {
    let xml = r#"<IccProfile><MF><EF id="6FC7"><SIMIO cmd="B2" p1="XX" p2="04" p3="26">144,0,01</SIMIO></EF></MF></IccProfile>"#;
    let result = parse_xml_profile(xml);
    assert!(matches!(result, Err(XmlProfileError::Deserialization(_))));
}

#[test]
fn test_invalid_ccid_fails() {
    let xml = r#"<IccProfile><MF><EF id="2FE2"><CCID>898603abc</CCID></EF></MF></IccProfile>"#;
    let result = parse_xml_profile(xml);
    assert!(matches!(result, Err(XmlProfileError::InvalidValue { field, .. }) if field == "CCID"));
}

#[test]
fn test_invalid_cimi_fails() {
    let xml = r#"<IccProfile><MF><ADF aid="A0000000871002FF86FF0389FFFFFFFF"><EF id="6F07"><CIMI>31174abc</CIMI></EF></ADF></MF></IccProfile>"#;
    let result = parse_xml_profile(xml);
    assert!(matches!(result, Err(XmlProfileError::InvalidValue { field, .. }) if field == "CIMI"));
}

#[test]
fn test_invalid_simio_response_fails() {
    let xml = r#"<IccProfile><MF><EF id="2FE2"><SIMIO cmd="B0" p1="0" p2="0" p3="A">144,0</SIMIO></EF></MF></IccProfile>"#;
    let result = parse_xml_profile(xml);
    assert!(matches!(result, Err(XmlProfileError::InvalidSimIoResponse { .. })));
}

#[test]
fn test_missing_mf_fails() {
    let xml = r#"<IccProfile></IccProfile>"#;
    let result = parse_xml_profile(xml);
    assert_eq!(result, Err(XmlProfileError::MissingMasterFile));
}

#[test]
fn test_parse_pin_retries() {
    let xml = r#"<IccProfile>
        <MF></MF>
        <PinProfile>
            <PINREMAINTIMES>5</PINREMAINTIMES>
            <PUKREMAINTIMES>15</PUKREMAINTIMES>
            <PIN2REMAINTIMES>6</PIN2REMAINTIMES>
            <PUK2REMAINTIMES>16</PUK2REMAINTIMES>
        </PinProfile>
    </IccProfile>"#;
    let profile = parse_xml_profile(xml).unwrap();
    assert_eq!(profile.pin_profile.pin1_retries, Some(5));
    assert_eq!(profile.pin_profile.puk1_retries, Some(15));
    assert_eq!(profile.pin_profile.pin2_retries, Some(6));
    assert_eq!(profile.pin_profile.puk2_retries, Some(16));
}

#[test]
fn test_parse_eid_and_atr() {
    let xml = r#"<IccProfile>
        <MF></MF>
        <CardProfile>
            <EID>89049032000001000000000254806852</EID>
            <ATR>3F979580BFFE8210428031A073BE211797</ATR>
        </CardProfile>
    </IccProfile>"#;
    let profile = parse_xml_profile(xml).unwrap();
    assert_eq!(profile.eid, Some("89049032000001000000000254806852".to_string()));
    assert_eq!(profile.atr, Some("3F979580BFFE8210428031A073BE211797".to_string()));
}

#[test]
fn test_parse_fcp_linear_fixed_c0_simio() {
    // Tag 0x82 len 5: 02 (linear fixed) 00 00 1C (rec_len = 28) 03 (num_records =
    // 3) Tag 0x80 len 2: 00 54 (file_size = 84 = 3 * 28)
    let xml = r#"<IccProfile>
        <MF>
            <EF id="4F33">
                <SIMIO cmd="C0" p1="0" p2="0" p3="1C">98,130,621982050200001C0383024F338A01058B036F060180020054</SIMIO>
            </EF>
        </MF>
    </IccProfile>"#;
    let profile = parse_xml_profile(xml).unwrap();
    let ef = profile.sim_io.file_system.find_ef(0x4F33u16).unwrap();
    assert_eq!(ef.record_len, Some(28));
    assert_eq!(ef.size(), 84);
    assert_eq!(ef.data, vec![0xFF; 84]);
}
