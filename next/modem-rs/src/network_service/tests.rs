// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use super::*;

#[test]
fn test_cops_query_registered_numeric() {
    let response = NetworkResponse::OperatorQuery {
        is_registered: true,
        mode: CopsMode::Automatic,
        format: CopsFormat::Numeric,
        plmn: "310260".to_string(),
        quirks: Quirks::default(),
    };
    assert_eq!(format!("{response}"), "+COPS: 0,2,\"310260\"\r\n");
}

#[test]
fn test_cops_query_unregistered_standard() {
    let quirks = Quirks { goldfish_ril_37_or_earlier: false, ..Default::default() };
    let response = NetworkResponse::OperatorQuery {
        is_registered: false,
        mode: CopsMode::Automatic,
        format: CopsFormat::Numeric,
        plmn: "310260".to_string(),
        quirks,
    };
    assert_eq!(format!("{response}"), "+COPS: 0\r\n");
}

#[test]
fn test_cops_query_unregistered_goldfish_37_compound_queries() {
    let quirks = Quirks { goldfish_ril_37_or_earlier: true, ..Default::default() };
    for (fmt, expected_fmt_code) in [
        (CopsFormat::LongAlphanumeric, "0"),
        (CopsFormat::ShortAlphanumeric, "1"),
        (CopsFormat::Numeric, "2"),
    ] {
        let response = NetworkResponse::OperatorQuery {
            is_registered: false,
            mode: CopsMode::Automatic,
            format: fmt,
            plmn: "310260".to_string(),
            quirks,
        };
        assert_eq!(format!("{response}"), format!("+COPS: 0,{expected_fmt_code},\"\"\r\n"));
    }
}

#[test]
fn test_radio_admission_truth_table() {
    let mut net = NetworkService::new(Quirks::default(), None);
    // Default: radio_power is Full, voice/data are NotRegistered.
    assert!(net.is_radio_on());

    assert_eq!(net.can_originate_voice_call(false), Err(AdmissionDenial::NoNetworkService));
    assert_eq!(net.can_originate_voice_call(true), Ok(()));
    assert_eq!(net.can_originate_data(), Err(AdmissionDenial::NoNetworkService));
    assert!(!net.can_terminate_voice_call());
    assert_eq!(net.can_originate_sms(), Err(AdmissionDenial::NoNetworkService));
    assert!(!net.can_terminate_sms());

    net.set_registration(RegistrationType::Voice, RegistrationStatus::Searching);
    net.set_registration(RegistrationType::Data, RegistrationStatus::Searching);
    assert_eq!(net.can_originate_voice_call(false), Err(AdmissionDenial::NoNetworkService));
    assert_eq!(net.can_originate_voice_call(true), Ok(()));
    assert_eq!(net.can_originate_data(), Err(AdmissionDenial::NoNetworkService));
    assert!(!net.can_terminate_voice_call());
    assert_eq!(net.can_originate_sms(), Err(AdmissionDenial::NoNetworkService));
    assert!(!net.can_terminate_sms());

    net.set_registration(RegistrationType::Voice, RegistrationStatus::Denied);
    net.set_registration(RegistrationType::Data, RegistrationStatus::Denied);
    assert_eq!(net.can_originate_voice_call(false), Err(AdmissionDenial::NoNetworkService));
    assert_eq!(net.can_originate_voice_call(true), Ok(()));
    assert_eq!(net.can_originate_data(), Err(AdmissionDenial::NoNetworkService));
    assert!(!net.can_terminate_voice_call());
    assert_eq!(net.can_originate_sms(), Err(AdmissionDenial::NoNetworkService));
    assert!(!net.can_terminate_sms());

    net.set_registration(RegistrationType::Voice, RegistrationStatus::Unknown);
    net.set_registration(RegistrationType::Data, RegistrationStatus::Unknown);
    assert_eq!(net.can_originate_voice_call(false), Err(AdmissionDenial::NoNetworkService));
    assert_eq!(net.can_originate_voice_call(true), Ok(()));
    assert_eq!(net.can_originate_data(), Err(AdmissionDenial::NoNetworkService));
    assert!(!net.can_terminate_voice_call());
    assert_eq!(net.can_originate_sms(), Err(AdmissionDenial::NoNetworkService));
    assert!(!net.can_terminate_sms());

    net.set_registration(RegistrationType::Voice, RegistrationStatus::Emergency);
    net.set_registration(RegistrationType::Data, RegistrationStatus::Emergency);
    assert_eq!(net.can_originate_voice_call(false), Err(AdmissionDenial::EmergencyCallsOnly));
    assert_eq!(net.can_originate_voice_call(true), Ok(()));
    assert_eq!(net.can_originate_data(), Err(AdmissionDenial::EmergencyCallsOnly));
    assert!(!net.can_terminate_voice_call());
    assert_eq!(net.can_originate_sms(), Err(AdmissionDenial::NoNetworkService));
    assert!(!net.can_terminate_sms());

    net.set_registration(RegistrationType::Voice, RegistrationStatus::RegisteredHome);
    net.set_registration(RegistrationType::Data, RegistrationStatus::RegisteredHome);
    assert_eq!(net.can_originate_voice_call(false), Ok(()));
    assert_eq!(net.can_originate_voice_call(true), Ok(()));
    assert_eq!(net.can_originate_data(), Ok(()));
    assert!(net.can_terminate_voice_call());
    assert_eq!(net.can_originate_sms(), Ok(()));
    assert!(net.can_terminate_sms());

    net.set_registration(RegistrationType::Voice, RegistrationStatus::Roaming);
    net.set_registration(RegistrationType::Data, RegistrationStatus::Roaming);
    assert_eq!(net.can_originate_voice_call(false), Ok(()));
    assert_eq!(net.can_originate_voice_call(true), Ok(()));
    assert_eq!(net.can_originate_data(), Ok(()));
    assert!(net.can_terminate_voice_call());
    assert_eq!(net.can_originate_sms(), Ok(()));
    assert!(net.can_terminate_sms());

    net.radio_power = RadioPowerLevel::Minimum;
    assert!(!net.is_radio_on());
    assert_eq!(net.can_originate_voice_call(false), Err(AdmissionDenial::NoNetworkService));
    assert_eq!(net.can_originate_voice_call(true), Err(AdmissionDenial::NoNetworkService));
    assert_eq!(net.can_originate_data(), Err(AdmissionDenial::NoNetworkService));
    assert!(!net.can_terminate_voice_call());
    assert_eq!(net.can_originate_sms(), Err(AdmissionDenial::NoNetworkService));
    assert!(!net.can_terminate_sms());

    net.radio_power = RadioPowerLevel::DisableRf;
    assert!(!net.is_radio_on());
    assert_eq!(net.can_originate_voice_call(false), Err(AdmissionDenial::NoNetworkService));
    assert_eq!(net.can_originate_voice_call(true), Err(AdmissionDenial::NoNetworkService));
    assert_eq!(net.can_originate_data(), Err(AdmissionDenial::NoNetworkService));
    assert!(!net.can_terminate_voice_call());
    assert_eq!(net.can_originate_sms(), Err(AdmissionDenial::NoNetworkService));
    assert!(!net.can_terminate_sms());
}
