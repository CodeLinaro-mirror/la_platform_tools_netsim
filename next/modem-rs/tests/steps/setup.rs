// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use hex;
use modem_rs::{
    DedicatedFile, ElementaryFile, FileSystem, PinState, SimFile, SimIo, SimProfile,
    config::PinProfile, constants::UiccFileId, test_utils::MockModemHandler,
};
use netsim_model::Quirks;

use crate::{common::constants::*, world::World};

/// Creates a modem with the given name.
///
/// # Panics
///
/// Panics if a modem with the same name already exists or if creation fails.
pub fn given_modem(world: &mut World, name: &str) {
    if world.modems.contains_key(name) {
        panic!("Modem with name '{name}' already exists");
    }

    let id = world.next_modem_id();
    let (handler, sink) = MockModemHandler::new(false);

    world
        .manager
        .new_modem(id, sink, None, None, Quirks::default())
        .expect("Failed to create new modem");
    world.modems.insert(name.to_string(), (id, handler));
}

/// Creates a Goldfish 37 (or earlier) modem with the given name.
pub fn given_goldfish_37_modem(world: &mut World, name: &str) {
    if world.modems.contains_key(name) {
        panic!("Modem with name '{name}' already exists");
    }

    let id = world.next_modem_id();
    let (handler, sink) = MockModemHandler::new(true);

    let quirks = Quirks { goldfish_ril_37_or_earlier: true, ..Default::default() };
    world.manager.new_modem(id, sink, None, None, quirks).expect("Failed to create new modem");
    world.modems.insert(name.to_string(), (id, handler));
}

/// Creates a modem with the given name and SIM type.
pub fn given_modem_with_sim_type(world: &mut World, name: &str, sim_type: i32) {
    if world.modems.contains_key(name) {
        panic!("Modem with name '{name}' already exists");
    }

    let id = world.next_modem_id();
    let (handler, sink) = MockModemHandler::new(false);

    world
        .manager
        .new_modem(id, sink, Some(sim_type), None, Quirks::default())
        .expect("Failed to create new modem");
    world.modems.insert(name.to_string(), (id, handler));
}

/// Creates a Cuttlefish modem with the given name.
pub fn given_cuttlefish_modem(world: &mut World, name: &str) {
    if world.modems.contains_key(name) {
        panic!("Modem with name '{name}' already exists");
    }

    let id = world.next_modem_id();
    let (handler, sink) = MockModemHandler::new(false);

    let quirks = Quirks { is_cuttlefish: true, ..Default::default() };
    world.manager.new_modem(id, sink, None, None, quirks).expect("Failed to create new modem");
    world.modems.insert(name.to_string(), (id, handler));
}

/// Creates a modem with the given name and phone number.
///
/// This first creates the modem, then sets its phone number via `ModemImpl`.
pub fn given_modem_with_number(world: &mut World, name: &str, number: &str) {
    given_modem(world, name);
    let (id, _) = world.get_modem(name);
    if let Some(modem) = world.manager.get_modem_mut(id) {
        modem.set_phone_number(number);
    } else {
        panic!("Failed to retrieve modem '{name}' after creation");
    }
}

/// Creates a modem with the default test SIM profile (legacy behavior).
///
/// This profile includes specific ICCID, IMSI ("123456789012345"), and a file
/// system with `2FE2`.
pub fn given_modem_with_sim_profile(world: &mut World, name: &str) {
    if world.modems.contains_key(name) {
        panic!("Modem with name '{name}' already exists");
    }

    let id = world.next_modem_id();
    let (handler, sink) = MockModemHandler::new(false);

    let profile = create_legacy_test_profile();

    world
        .manager
        .new_modem_with_profile(id, sink, Some(profile), None, Quirks::default())
        .expect("Failed to create modem with profile");
    world.modems.insert(name.to_string(), (id, handler));
}

/// Creates a modem with a custom XML SIM profile.
pub fn given_modem_with_xml_profile(world: &mut World, name: &str, xml: &str) {
    if world.modems.contains_key(name) {
        panic!("Modem with name '{name}' already exists");
    }

    let id = world.next_modem_id();
    let (handler, sink) = MockModemHandler::new(false);

    world
        .manager
        .new_modem(id, sink, None, Some(xml.to_string()), Quirks::default())
        .expect("Failed to create new modem with XML profile");
    world.modems.insert(name.to_string(), (id, handler));
}

/// Helper function to create the legacy SIM profile used in tests.
pub fn create_legacy_test_profile() -> SimProfile {
    SimProfile {
        iccid: TEST_ICCID.to_string(),
        imsi: TEST_IMSI.to_string(),
        sim_io: SimIo {
            file_system: FileSystem {
                master_file: DedicatedFile {
                    file_id: UiccFileId::MasterFile.into(),
                    files: vec![SimFile::ElementaryFile(ElementaryFile {
                        file_id: UiccFileId::Iccid.into(),

                        record_len: None,
                        data: hex::decode(TEST_ICCID).unwrap(),
                    })],
                },
            },
        },
        ..Default::default()
    }
}

/// Helper function to create a SIM profile with PIN lock enabled but not
/// verified.
pub fn create_locked_sim_profile() -> SimProfile {
    SimProfile {
        iccid: TEST_ICCID.to_string(),
        imsi: TEST_IMSI.to_string(),
        pin_profile: PinProfile {
            state: PinState::EnabledNotVerified,
            pin1: LOCKED_PIN.to_string(),
            puk1: TEST_PUK.to_string(),
            ..Default::default()
        },
        sim_io: SimIo {
            file_system: FileSystem {
                master_file: DedicatedFile {
                    file_id: UiccFileId::MasterFile.into(),
                    files: vec![SimFile::ElementaryFile(ElementaryFile {
                        file_id: UiccFileId::Iccid.into(),

                        record_len: None,
                        data: hex::decode(TEST_ICCID).unwrap(),
                    })],
                },
            },
        },
        ..Default::default()
    }
}

/// Creates a modem with a locked SIM profile (legacy test profile with PIN
/// EnabledNotVerified).
pub fn given_modem_with_locked_sim(world: &mut World, name: &str) {
    if world.modems.contains_key(name) {
        panic!("Modem with name '{name}' already exists");
    }

    let id = world.next_modem_id();
    let (handler, sink) = MockModemHandler::new(false);

    let profile = create_locked_sim_profile();

    world
        .manager
        .new_modem_with_profile(id, sink, Some(profile), None, Quirks::default())
        .expect("Failed to create modem with locked SIM");
    world.modems.insert(name.to_string(), (id, handler));
}

/// Helper function to create a SIM profile with PIN permanently blocked.
pub fn create_perm_blocked_sim_profile() -> SimProfile {
    SimProfile {
        iccid: TEST_ICCID.to_string(),
        imsi: TEST_IMSI.to_string(),
        pin_profile: PinProfile {
            state: PinState::PermBlocked,
            pin1: LOCKED_PIN.to_string(),
            puk1: TEST_PUK.to_string(),
            puk1_retries: Some(0),
            ..Default::default()
        },
        sim_io: SimIo {
            file_system: FileSystem {
                master_file: DedicatedFile {
                    file_id: UiccFileId::MasterFile.into(),
                    files: vec![SimFile::ElementaryFile(ElementaryFile {
                        file_id: UiccFileId::Iccid.into(),

                        record_len: None,
                        data: hex::decode(TEST_ICCID).unwrap(),
                    })],
                },
            },
        },
        ..Default::default()
    }
}

/// Creates a modem with a permanently blocked SIM profile.
pub fn given_modem_with_perm_blocked_sim(world: &mut World, name: &str) {
    if world.modems.contains_key(name) {
        panic!("Modem with name '{name}' already exists");
    }

    let id = world.next_modem_id();
    let (handler, sink) = MockModemHandler::new(false);

    let profile = create_perm_blocked_sim_profile();

    world
        .manager
        .new_modem_with_profile(id, sink, Some(profile), None, Quirks::default())
        .expect("Failed to create modem with perm blocked SIM");
    world.modems.insert(name.to_string(), (id, handler));
}

/// Helper function to create a SIM profile with EF_MSISDN in the filesystem.
pub fn create_profile_with_msisdn() -> SimProfile {
    SimProfile {
        iccid: TEST_ICCID.to_string(),
        imsi: TEST_IMSI.to_string(),
        sim_io: SimIo {
            file_system: FileSystem {
                master_file: DedicatedFile {
                    file_id: UiccFileId::MasterFile.into(),
                    files: vec![
                        SimFile::ElementaryFile(ElementaryFile {
                            file_id: UiccFileId::Iccid.into(),

                            record_len: None,
                            data: hex::decode(TEST_ICCID).unwrap(),
                        }),
                        SimFile::DedicatedFile(DedicatedFile {
                            file_id: UiccFileId::Telecom.into(),
                            files: vec![SimFile::ElementaryFile(ElementaryFile {
                                file_id: UiccFileId::Msisdn.into(),

                                record_len: UiccFileId::Msisdn.default_record_len(),
                                data: vec![
                                    0xFF;
                                    UiccFileId::Msisdn
                                        .default_record_len()
                                        .expect("file id is record based")
                                ],
                            })],
                        }),
                    ],
                },
            },
        },
        ..Default::default()
    }
}

/// Creates a modem with a SIM profile that has EF_MSISDN in the filesystem.
pub fn given_modem_with_msisdn_in_fs(world: &mut World, name: &str) {
    if world.modems.contains_key(name) {
        panic!("Modem with name '{name}' already exists");
    }

    let id = world.next_modem_id();
    let (handler, sink) = MockModemHandler::new(false);

    let profile = create_profile_with_msisdn();

    world
        .manager
        .new_modem_with_profile(id, sink, Some(profile), None, Quirks::default())
        .expect("Failed to create modem with MSISDN in FS");
    world.modems.insert(name.to_string(), (id, handler));
}

/// Helper function to create a SIM profile with a 2-record EF_MSISDN in the
/// filesystem.
pub fn create_profile_with_multi_record_msisdn() -> SimProfile {
    let line1 = hex::decode("4C696E6531FFFFFFFFFFFFFFFFFF07915155111111F1FFFFFFFFFFFF").unwrap();
    let line2 = hex::decode("4C696E6532FFFFFFFFFFFFFFFFFF07915155222222F2FFFFFFFFFFFF").unwrap();
    let mut data = Vec::new();
    data.extend(line1);
    data.extend(line2);

    SimProfile {
        iccid: TEST_ICCID.to_string(),
        imsi: TEST_IMSI.to_string(),
        sim_io: SimIo {
            file_system: FileSystem {
                master_file: DedicatedFile {
                    file_id: UiccFileId::MasterFile.into(),
                    files: vec![
                        SimFile::ElementaryFile(ElementaryFile {
                            file_id: UiccFileId::Iccid.into(),
                            record_len: None,
                            data: hex::decode(TEST_ICCID).unwrap(),
                        }),
                        SimFile::DedicatedFile(DedicatedFile {
                            file_id: UiccFileId::Telecom.into(),
                            files: vec![SimFile::ElementaryFile(ElementaryFile {
                                file_id: UiccFileId::Msisdn.into(),
                                record_len: UiccFileId::Msisdn.default_record_len(),
                                data,
                            })],
                        }),
                    ],
                },
            },
        },
        ..Default::default()
    }
}

/// Creates a modem with a SIM profile that has a 2-record EF_MSISDN in the
/// filesystem.
pub fn given_modem_with_multi_record_msisdn_in_fs(world: &mut World, name: &str) {
    if world.modems.contains_key(name) {
        panic!("Modem with name '{name}' already exists");
    }

    let id = world.next_modem_id();
    let (handler, sink) = MockModemHandler::new(false);
    let profile = create_profile_with_multi_record_msisdn();

    world
        .manager
        .new_modem_with_profile(id, sink, Some(profile), None, Quirks::default())
        .expect("Failed to create modem with multi-record MSISDN in FS");
    world.modems.insert(name.to_string(), (id, handler));
}

/// Helper function to create a SIM profile with a custom record length
/// EF_MSISDN.
pub fn create_profile_with_custom_record_len_msisdn(record_len: usize) -> SimProfile {
    SimProfile {
        iccid: TEST_ICCID.to_string(),
        imsi: TEST_IMSI.to_string(),
        sim_io: SimIo {
            file_system: FileSystem {
                master_file: DedicatedFile {
                    file_id: UiccFileId::MasterFile.into(),
                    files: vec![
                        SimFile::ElementaryFile(ElementaryFile {
                            file_id: UiccFileId::Iccid.into(),
                            record_len: None,
                            data: hex::decode(TEST_ICCID).unwrap(),
                        }),
                        SimFile::DedicatedFile(DedicatedFile {
                            file_id: UiccFileId::Telecom.into(),
                            files: vec![SimFile::ElementaryFile(ElementaryFile {
                                file_id: UiccFileId::Msisdn.into(),
                                record_len: Some(record_len),
                                data: vec![0xFF; record_len],
                            })],
                        }),
                    ],
                },
            },
        },
        ..Default::default()
    }
}

/// Creates a modem with a SIM profile that has a custom record length EF_MSISDN
/// in the filesystem.
pub fn given_modem_with_custom_record_len_msisdn_in_fs(
    world: &mut World,
    name: &str,
    record_len: usize,
) {
    if world.modems.contains_key(name) {
        panic!("Modem with name '{name}' already exists");
    }

    let id = world.next_modem_id();
    let (handler, sink) = MockModemHandler::new(false);
    let profile = create_profile_with_custom_record_len_msisdn(record_len);

    world
        .manager
        .new_modem_with_profile(id, sink, Some(profile), None, Quirks::default())
        .expect("Failed to create modem with custom record len MSISDN in FS");
    world.modems.insert(name.to_string(), (id, handler));
}

/// Helper function to create a SIM profile with EF_FPLMN and EF_MBDN in the
/// filesystem.
pub fn create_profile_with_fplmn_and_mbdn() -> SimProfile {
    SimProfile {
        iccid: TEST_ICCID.to_string(),
        imsi: TEST_IMSI.to_string(),
        sim_io: SimIo {
            file_system: FileSystem {
                master_file: DedicatedFile {
                    file_id: UiccFileId::MasterFile.into(),
                    files: vec![
                        SimFile::ElementaryFile(ElementaryFile {
                            file_id: UiccFileId::Iccid.into(),

                            record_len: None,
                            data: hex::decode(TEST_ICCID).unwrap(),
                        }),
                        SimFile::ElementaryFile(ElementaryFile {
                            file_id: UiccFileId::ForbiddenPlmn.into(),

                            record_len: None,
                            data: vec![0xFF; 12],
                        }),
                        SimFile::DedicatedFile(DedicatedFile {
                            file_id: UiccFileId::Telecom.as_u16(),
                            files: vec![SimFile::ElementaryFile(ElementaryFile {
                                file_id: UiccFileId::MailboxDialingNumbers.into(),
                                record_len: UiccFileId::MailboxDialingNumbers.default_record_len(),
                                data: vec![
                                    0xFF;
                                    4 * UiccFileId::MailboxDialingNumbers
                                        .default_record_len()
                                        .expect("file id is record based")
                                ],
                            })],
                        }),
                    ],
                },
            },
        },
        ..Default::default()
    }
}

/// Creates a modem with a SIM profile that has EF_FPLMN and EF_MBDN in the
/// filesystem.
pub fn given_modem_with_fplmn_and_mbdn_in_fs(world: &mut World, name: &str) {
    if world.modems.contains_key(name) {
        panic!("Modem with name '{name}' already exists");
    }

    let id = world.next_modem_id();
    let (handler, sink) = MockModemHandler::new(false);

    let profile = create_profile_with_fplmn_and_mbdn();

    world
        .manager
        .new_modem_with_profile(id, sink, Some(profile), None, Quirks::default())
        .expect("Failed to create modem with FPLMN and MBDN in FS");
    world.modems.insert(name.to_string(), (id, handler));
}

/// Helper function to create a SIM profile with FDN records.
pub fn create_fdn_sim_profile() -> SimProfile {
    let fdn_record1 =
        hex::decode("FFFFFFFFFFFFFFFFFFFFFFFFFFFF04812143F5FFFFFFFFFFFFFFFFFF").unwrap();
    let fdn_record2 =
        hex::decode("FFFFFFFFFFFFFFFFFFFFFFFFFFFF07916105550501F0FFFFFFFFFFFF").unwrap();
    let fdn_record3 =
        hex::decode("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF").unwrap();
    let mut fdn_data = Vec::new();
    fdn_data.extend(fdn_record1);
    fdn_data.extend(fdn_record2);
    fdn_data.extend(fdn_record3);

    SimProfile {
        iccid: TEST_ICCID.to_string(),
        imsi: TEST_IMSI.to_string(),
        pin_profile: PinProfile { pin2: "5678".to_string(), ..Default::default() },
        sim_io: SimIo {
            file_system: FileSystem {
                master_file: DedicatedFile {
                    file_id: UiccFileId::MasterFile.into(),
                    files: vec![
                        SimFile::ElementaryFile(ElementaryFile {
                            file_id: UiccFileId::Iccid.into(),
                            record_len: None,
                            data: hex::decode(TEST_ICCID).unwrap(),
                        }),
                        SimFile::DedicatedFile(DedicatedFile {
                            file_id: UiccFileId::Telecom.into(),
                            files: vec![SimFile::ElementaryFile(ElementaryFile {
                                file_id: UiccFileId::FixedDialingNumbers.into(),
                                record_len: UiccFileId::FixedDialingNumbers.default_record_len(),
                                data: fdn_data,
                            })],
                        }),
                    ],
                },
            },
        },
        ..Default::default()
    }
}

/// Creates a modem with a SIM profile that has FDN records.
pub fn given_modem_with_fdn_sim_profile(world: &mut World, name: &str) {
    if world.modems.contains_key(name) {
        panic!("Modem with name '{name}' already exists");
    }

    let id = world.next_modem_id();
    let (handler, sink) = MockModemHandler::new(false);

    let profile = create_fdn_sim_profile();

    world
        .manager
        .new_modem_with_profile(id, sink, Some(profile), None, Quirks::default())
        .expect("Failed to create modem with FDN SIM profile");
    world.modems.insert(name.to_string(), (id, handler));
}

/// Creates a 2G GSM SIM profile with MF (0x3F00) and DF_TELECOM (0x7F10)
/// without any Application Dedicated Files (ADFs).
pub fn create_2g_sim_profile() -> SimProfile {
    SimProfile {
        iccid: TEST_ICCID.to_string(),
        imsi: TEST_IMSI.to_string(),
        sim_io: SimIo {
            file_system: FileSystem {
                master_file: DedicatedFile {
                    file_id: UiccFileId::MasterFile.into(),
                    files: vec![
                        SimFile::ElementaryFile(ElementaryFile {
                            file_id: UiccFileId::Iccid.into(),
                            record_len: None,
                            data: hex::decode(TEST_ICCID).unwrap(),
                        }),
                        SimFile::DedicatedFile(DedicatedFile {
                            file_id: UiccFileId::Telecom.into(),
                            files: Vec::new(),
                        }),
                    ],
                },
            },
        },
        ..Default::default()
    }
}

/// Creates a modem with a 2G SIM profile (no ADFs).
pub fn given_modem_with_2g_sim_profile(world: &mut World, name: &str) {
    if world.modems.contains_key(name) {
        panic!("Modem with name '{name}' already exists");
    }

    let id = world.next_modem_id();
    let (handler, sink) = MockModemHandler::new(false);

    let profile = create_2g_sim_profile();

    world
        .manager
        .new_modem_with_profile(id, sink, Some(profile), None, Quirks::default())
        .expect("Failed to create modem with 2G SIM profile");
    world.modems.insert(name.to_string(), (id, handler));
}
