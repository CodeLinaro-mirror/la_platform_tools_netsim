// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;

use super::{conversion::normalize_command, *};
use crate::{config::ApduMapping, profiles::Profile};

#[test]
fn test_parse_default_profile() {
    let profile = parse_xml_profile(crate::profiles::PROFILE_DEFAULT_XML);
    assert!(profile.is_ok(), "Failed to parse default profile: {:?}", profile.err());
    let profile = profile.unwrap();
    assert_eq!(profile.imsi, "311740123456789");
    assert_eq!(profile.iccid, "89860318640220133897");
}

#[test]
fn test_parse_cts_profile() {
    let profile = parse_xml_profile(crate::profiles::PROFILE_CTS_XML);
    assert!(profile.is_ok(), "Failed to parse CTS profile: {:?}", profile.err());
    let profile = profile.unwrap();
    assert_eq!(profile.imsi, "310260000000000");
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
            file_id: 0x6FC7,
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
    let ef = crate::sim_service::find_ef(&profile.sim_io.file_system.master_file, 0x6FC7).unwrap();
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
    let ef = crate::sim_service::find_ef(&profile.sim_io.file_system.master_file, 0x6FC7).unwrap();
    assert_eq!(ef.record_len, Some(38)); // 0x26 = 38
    assert_eq!(ef.size(), 76); // 2 records * 38 bytes
    assert_eq!(ef.data, vec![0xFF; 76]);
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

fn cgla_to_map(mappings: &[crate::profiles::CglaMapping]) -> HashMap<String, String> {
    mappings.iter().map(|m| (normalize_command(&m.cmd), m.response.to_ascii_uppercase())).collect()
}

fn apdu_to_map(mappings: &[ApduMapping]) -> HashMap<String, String> {
    mappings.iter().map(|m| (normalize_command(&m.cmd), m.response.to_ascii_uppercase())).collect()
}

fn count_efs(df: &crate::config::DedicatedFile) -> usize {
    df.files
        .iter()
        .map(|f| match f {
            crate::config::SimFile::ElementaryFile(_) => 1,
            crate::config::SimFile::DedicatedFile(sub_df) => count_efs(sub_df),
        })
        .sum()
}

fn assert_profile_equivalence(static_prof: &Profile, dynamic_prof: &SimProfile) {
    // 1. Verify Filesystem (simio_files)
    for fm in static_prof.simio_files {
        let fid = u16::from_str_radix(fm.file_id, 16).expect("Static file_id should be valid hex");
        let ef = crate::sim_service::find_ef(&dynamic_prof.sim_io.file_system.master_file, fid)
            .unwrap_or_else(|| panic!("EF {fid:04X} not found in dynamic profile"));

        // Check if there is a B0 (read binary) mapping
        if let Some(b0_map) = fm.mappings.iter().find(|m| m.cmd == "B0") {
            let mut parts = b0_map.response.split(',');
            if let (Some(_sw1), Some(_sw2), Some(payload), None) =
                (parts.next(), parts.next(), parts.next(), parts.next())
            {
                let expected_data = payload.trim().to_ascii_uppercase();
                let expected_bytes = hex::decode(&expected_data).unwrap();
                assert_eq!(ef.data, expected_bytes, "Data mismatch for EF {fid:04X} (B0)");
                assert_eq!(
                    ef.record_len, None,
                    "Transparent EF {fid:04X} should have record_len = None"
                );
            } else {
                panic!("Invalid static B0 mapping response for EF {fid:04X}");
            }
        }
        // Check if there are B2 (read record) mappings
        else if fm.mappings.iter().any(|m| m.cmd == "B2") {
            let mut records: Vec<(usize, String)> = Vec::new();
            for m in fm.mappings {
                if m.cmd == "B2" {
                    let rec_num = usize::from_str_radix(m.p1, 16)
                        .expect("Static record number should be valid hex");
                    let mut parts = m.response.split(',');
                    if let (Some(_sw1), Some(_sw2), Some(payload), None) =
                        (parts.next(), parts.next(), parts.next(), parts.next())
                    {
                        records.push((rec_num, payload.trim().to_ascii_uppercase()));
                    } else {
                        panic!("Invalid static B2 mapping response for EF {fid:04X}");
                    }
                }
            }
            records.sort_by_key(|r| r.0);
            let expected_rec_len = records.first().map(|r| r.1.len() / 2);
            let expected_data: String = records.into_iter().map(|r| r.1).collect();
            let expected_bytes = hex::decode(&expected_data).unwrap();
            assert_eq!(ef.data, expected_bytes, "Data mismatch for EF {fid:04X} (B2)");
            assert_eq!(
                ef.record_len, expected_rec_len,
                "Record length mismatch for linear fixed EF {fid:04X}"
            );
        } else {
            assert!(
                ef.data.is_empty() || ef.data.iter().all(|&b| b == 0xFF),
                "EF {fid:04X} without read mappings should have empty data or default 'FF' padding, found: {:?}",
                ef.data
            );
        }
    }

    assert_eq!(
        count_efs(&dynamic_prof.sim_io.file_system.master_file),
        static_prof.simio_files.len() + 1,
        "EF count mismatch: dynamic profile has all static simio_files plus EF_IMSI (6F07)"
    );

    // 2. Verify ADFs
    assert_eq!(static_prof.adfs.len(), dynamic_prof.adfs.len(), "ADF count mismatch");

    for adf_mock in static_prof.adfs {
        let dynamic_adf = dynamic_prof
            .adfs
            .iter()
            .find(|a| a.aid.eq_ignore_ascii_case(adf_mock.aid))
            .unwrap_or_else(|| {
                panic!("ADF with AID '{}' not found in dynamic profile", adf_mock.aid)
            });

        // Compare ADF-level CGLA
        assert_eq!(
            apdu_to_map(&dynamic_adf.cgla),
            cgla_to_map(adf_mock.cgla),
            "ADF-level CGLA mismatch for AID '{}'",
            adf_mock.aid
        );

        // Compare ADF-level CSIM
        assert_eq!(
            apdu_to_map(&dynamic_adf.csim),
            cgla_to_map(adf_mock.csim),
            "ADF-level CSIM mismatch for AID '{}'",
            adf_mock.aid
        );

        // Compare file overrides
        assert_eq!(
            file_mock_count_static(adf_mock),
            dynamic_adf.files.len(),
            "File override count mismatch in ADF '{}'",
            adf_mock.aid
        );

        for file_mock in adf_mock.files {
            let dynamic_override =
                dynamic_adf.files.iter().find(|f| f.id == file_mock.id).unwrap_or_else(|| {
                    panic!(
                        "Override for file {:04X} not found in dynamic ADF {}",
                        file_mock.id, adf_mock.aid
                    )
                });

            assert_eq!(
                apdu_to_map(&dynamic_override.cgla),
                cgla_to_map(file_mock.cgla),
                "CGLA mapping mismatch for file {:04X} in ADF {}",
                file_mock.id,
                adf_mock.aid
            );
        }
    }
}

fn file_mock_count_static(adf: &crate::profiles::AdfMock) -> usize {
    adf.files.len()
}

#[test]
fn test_default_profile_equivalence() {
    let dynamic_prof = parse_xml_profile(crate::profiles::PROFILE_DEFAULT_XML).unwrap();
    assert_profile_equivalence(&crate::profiles::PROFILE_DEFAULT, &dynamic_prof);
}

#[test]
fn test_cts_profile_equivalence() {
    let dynamic_prof = parse_xml_profile(crate::profiles::PROFILE_CTS_XML).unwrap();
    assert_profile_equivalence(&crate::profiles::PROFILE_CTS, &dynamic_prof);
}
