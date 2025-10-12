// Copyright 2023-2025 The Android Open Source Project

use super::types::*;
use log::debug;
use rootcanal::{
    bluetooth::{Bluetooth, Callbacks as RootcanalCallbacks},
    types::Phy,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, Mutex as TokioMutex};
use tokio::time::{interval, Duration};

/// The `BluetoothManager` is the central component of the Bluetooth simulation.
///
/// It is responsible for:
/// - Managing the lifecycle of all simulated Bluetooth chips.
/// - Handling commands to create, patch, get, and delete chips.
/// - Running the main event loop that drives the simulation.
/// - Interacting with the `rootcanal` backend.
pub struct BluetoothManager {
    pub(crate) rootcanal: Arc<Bluetooth>,
    pub(crate) chips: Mutex<HashMap<u32, ChipEntry>>,
    pub(crate) command_rx: TokioMutex<mpsc::Receiver<BluetoothCommand>>,
    pub(crate) chip_death_rx: TokioMutex<mpsc::Receiver<ChipDied>>,
    pub(crate) chip_death_tx: mpsc::Sender<ChipDied>,
}

struct RootcanalCallbacksImpl;

impl RootcanalCallbacks for RootcanalCallbacksImpl {
    fn on_send_ll(
        &self,
        _source_id: u32,
        _destination_id: u32,
        _packet: &[u8],
        _phy: Phy,
        tx_power: i32,
    ) -> Option<i32> {
        Some(tx_power)
    }
}

impl BluetoothManager {
    /// Creates a new `BluetoothManager` and returns a tuple containing the
    /// manager and a channel for sending commands to it.
    pub fn new() -> (Self, mpsc::Sender<BluetoothCommand>) {
        let (command_tx, command_rx) = mpsc::channel(10);
        let (chip_death_tx, chip_death_rx) = mpsc::channel(10);
        let manager = BluetoothManager {
            rootcanal: Bluetooth::new(Box::new(RootcanalCallbacksImpl {})),
            chips: Mutex::new(HashMap::new()),
            command_rx: TokioMutex::new(command_rx),
            chip_death_rx: TokioMutex::new(chip_death_rx),
            chip_death_tx,
        };
        (manager, command_tx)
    }

    /// Returns a handle to the `rootcanal` instance for testing purposes.
    #[cfg(feature = "test-utils")]
    pub fn rootcanal_for_testing(&self) -> Arc<Bluetooth> {
        self.rootcanal.clone()
    }

    /// Returns the number of active chips for testing purposes.
    #[cfg(feature = "test-utils")]
    pub fn get_chip_count_for_testing(&self) -> usize {
        self.chips.lock().unwrap().len()
    }

    /// Runs a single iteration of the event loop for testing purposes.
    #[cfg(feature = "test-utils")]
    pub async fn run_once_for_testing(&self) {
        self.rootcanal.tick();
        tokio::select! {
            Some(cmd) = async { self.command_rx.lock().await.recv().await } => {
                self.handle_command(cmd);
            }
            Some(death_notice) = async { self.chip_death_rx.lock().await.recv().await } => {
                self.handle_chip_death(death_notice);
            }
            else => {}
        }
    }

    /// Runs the main event loop of the `BluetoothManager`.
    ///
    /// This function continuously processes incoming commands and events.
    pub async fn run(&self) {
        let mut tick_interval = interval(Duration::from_millis(10));
        loop {
            tokio::select! {
                _ = tick_interval.tick() => {
                    self.rootcanal.tick();
                }
                Some(cmd) = async { self.command_rx.lock().await.recv().await } => {
                    self.handle_command(cmd);
                }
                Some(death_notice) = async { self.chip_death_rx.lock().await.recv().await } => {
                    self.handle_chip_death(death_notice);
                }
            }
        }
    }

    fn handle_chip_death(&self, notice: ChipDied) {
        debug!("Handling chip death for chip_id: {}", notice.chip_id);
        println!("Chip {}'s packet stream died, cleaning up.", notice.chip_id);
        self.chips.lock().unwrap().remove(&notice.chip_id);
        let _ = self.rootcanal.remove_controller(notice.chip_id);
    }
}
