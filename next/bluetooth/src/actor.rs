// Copyright 2025 The Android Open Source Project

use crate::error::BluetoothError;
use crate::handlers::events::on_stream;
use crate::ranging;
use actor_framework::{ActorLifecycle, Context, StreamMessage};
use async_trait::async_trait;
use client::DeviceClient;
use netsim_model::chip::{Chip, ChipId};
use rootcanal::{Callbacks as RootcanalCallbacks, Phy, Rootcanal};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A thread-safe map of chip states.
pub type ChipMap = Arc<Mutex<HashMap<ChipId, Chip>>>;

/// Implementation of `RootcanalCallbacks` for the Bluetooth Actor.
/// We need to wrap this in a Mutex even though Rootcanal runs on
/// the same thread because RootcanalCallbacks is not sync.
struct RootcanalCallbacksImpl {
    chips: ChipMap,
}

impl RootcanalCallbacks for RootcanalCallbacksImpl {
    fn on_send_ll(
        &self,
        source_id: u32,
        destination_id: u32,
        _packet: &[u8],
        _phy: Phy,
        tx_power: i32,
    ) -> Option<i32> {
        let src_id = source_id.into();
        let dst_id = destination_id.into();
        let chips = self.chips.lock().unwrap();
        let src_chip = chips.get(&src_id);
        let dst_chip = chips.get(&dst_id);

        if let (Some(src), Some(dst)) = (src_chip, dst_chip) {
            let dist = ranging::distance(&src.position, &dst.position);
            // TODO: check for src_chip's link to dst_chip's RSSI override
            let rssi = ranging::distance_to_rssi(tx_power as i8, dist);
            Some(rssi as i32)
        } else {
            // If one of the chips is missing, default to tx_power.
            // This can happen during startup/shutdown or if a chip is not yet fully registered.
            Some(tx_power)
        }
    }
}

/// The context for the Bluetooth actor.
///
/// This struct holds the shared state required by the actor, including the
/// `Rootcanal` instance for simulation, the map of active chips, and the
/// `DeviceClient` for interacting with other devices.
pub struct BluetoothActor {
    /// The Rootcanal simulation instance.
    pub rootcanal: Arc<Rootcanal>,
    /// A map of active Bluetooth chips, protected by a mutex.
    pub chips: ChipMap,
    /// The client for interacting with the device actor.
    pub device_client: DeviceClient,
    /// Client to send actions/delete requests to the actor itself.
    pub client: Option<actor_framework::ResourceClient<crate::service::BluetoothEntity>>,
}

impl BluetoothActor {
    /// Creates a new BluetoothActor context.
    pub fn new(device_client: DeviceClient) -> Self {
        let chips = Arc::new(Mutex::new(HashMap::new()));
        let rootcanal = Rootcanal::new(Box::new(RootcanalCallbacksImpl { chips: chips.clone() }));
        Self { rootcanal, chips, device_client, client: None }
    }
}

#[async_trait]
impl ActorLifecycle for BluetoothActor {
    type Error = BluetoothError;

    async fn on_start(&mut self, runtime: &mut impl Context) {
        // Tick every 10ms to drive Rootcanal
        runtime.set_interval(Duration::from_millis(10));
    }

    async fn on_tick(&mut self, _runtime: &mut impl Context) {
        self.rootcanal.tick();
    }

    // TODO: Check if these can be consolidated.
    async fn on_stream(&mut self, id: usize, message: StreamMessage, runtime: &mut impl Context) {
        on_stream(self, id, message, runtime).await;
    }

    async fn on_stream_closed(&mut self, id: usize) -> Result<bool, Self::Error> {
        log::info!("Stream closed for chip {id}");
        Ok(true)
    }

    async fn on_task_closed(&mut self, id: usize) -> Result<bool, Self::Error> {
        log::info!("Task closed for chip {id}");
        Ok(true) // Request deletion of the chip
    }
}
