use std::sync::Arc;

use modem_rs::{
    config::{DedicatedFile, ElementaryFile, FileSystem, SimFile, SimIo, SimProfile},
    test_utils::MockModemHandler,
};

use crate::world::World;

/// Creates a modem with the given name.
///
/// # Panics
///
/// Panics if a modem with the same name already exists or if creation fails.
pub fn given_modem(world: &mut World, name: &str) {
    if world.modems.contains_key(name) {
        panic!("Modem with name '{}' already exists", name);
    }

    let id = world.next_modem_id();
    let handler = Arc::new(MockModemHandler::new());

    world.manager.new_modem(id, handler.clone()).expect("Failed to create new modem");
    world.modems.insert(name.to_string(), (id, handler));
}

/// Creates a modem with the given name and phone number.
///
/// This first creates the modem, then sets its phone number via `ModemImpl`.
pub fn given_modem_with_number(world: &mut World, name: &str, number: &str) {
    given_modem(world, name);
    let (id, _) = world.get_modem(name);
    if let Some(modem) = world.manager.get_modem(id) {
        modem.set_phone_number(number);
    } else {
        panic!("Failed to retrieve modem '{}' after creation", name);
    }
}

/// Creates a modem with the default test SIM profile (legacy behavior).
///
/// This profile includes specific ICCID, IMSI ("123456789012345"), and a file
/// system with `2FE2`.
pub fn given_modem_with_sim_profile(world: &mut World, name: &str) {
    if world.modems.contains_key(name) {
        panic!("Modem with name '{}' already exists", name);
    }

    let id = world.next_modem_id();
    let handler = Arc::new(MockModemHandler::new());

    let profile = create_legacy_test_profile();

    world
        .manager
        .new_modem_with_profile(id, handler.clone(), Some(profile))
        .expect("Failed to create modem with profile");
    world.modems.insert(name.to_string(), (id, handler));
}

/// Helper function to create the legacy SIM profile used in tests.
fn create_legacy_test_profile() -> SimProfile {
    SimProfile {
        iccid: "89012345678901234567".to_string(),
        imsi: "123456789012345".to_string(),
        sim_io: SimIo {
            file_system: FileSystem {
                master_file: DedicatedFile {
                    file_id: "3F00".to_string(),
                    files: vec![SimFile::Ef(ElementaryFile {
                        file_id: "2FE2".to_string(),
                        size: 10,
                        data: "89012345678901234567".to_string(),
                    })],
                },
            },
        },
        ..Default::default()
    }
}
