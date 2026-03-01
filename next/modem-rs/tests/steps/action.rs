use std::time::Duration;

use crate::world::World;

/// Sends an AT command string to the specified modem.
///
/// This function automatically appends `\r\n` if the command string does not
/// already end with it. The command is sent as bytes.
///
/// # Panics
///
/// Panics if the modem name is not found.
pub fn when_at_command_sent(world: &World, name: &str, command: &str) {
    let (id, _) = world.get_modem(name);
    // Ensure command ends with \r\n
    // Using Cow or simple checks to avoid allocation if possible would be better,
    // but tests typically use String for convenience.
    let full_command =
        if command.ends_with("\r\n") { command.to_string() } else { format!("{}\r\n", command) };
    world.manager.send_at_command(id, full_command.as_bytes());
}

/// Sends raw bytes, specified as a hex string, to the modem.
///
/// This is useful for sending non-ASCII sequences like PDU bytes or Ctrl-Z.
///
/// # Panics
///
/// Panics if the hex string is invalid or the modem is not found.
pub fn when_hex_bytes_sent(world: &World, name: &str, hex_bytes: &str) {
    let (id, _) = world.get_modem(name);
    let bytes = hex::decode(hex_bytes).expect("Invalid hex string provided to when_hex_bytes_sent");
    world.manager.send_at_command(id, &bytes);
}

/// Advances the simulated time by `ms` milliseconds and ticks the manager.
///
/// This ensures any time-dependent events (like timeouts) are processed.
pub fn when_time_advances_ms(world: &World, ms: u64) {
    world.clock.advance(Duration::from_millis(ms));
    // Ticking allows the manager to process expired timers immediately.
    world.manager.tick();
}

/// Triggers a physical channel configuration update on the modem.
///
/// This invokes the `on_update_physical_channel_configs` method on the modem's
/// data service, simulating network-side changes.
pub fn when_physical_channel_configs_updated(world: &World, name: &str) {
    let (id, _) = world.get_modem(name);
    if let Some(modem) = world.manager.get_modem(id) {
        modem.data_service.on_update_physical_channel_configs(&modem);
    } else {
        // This should theoretically be unreachable if get_modem succeeded,
        // but robust tests handle edge cases.
        panic!("Modem '{}' vanished after ID lookup!", name);
    }
}
