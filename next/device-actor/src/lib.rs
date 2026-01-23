// Copyright 2025 The Android Open Source Project

//! Device Actor
//!
//! This crate provides the `DeviceActor` which manages the lifecycle of devices
//! and their associated chips.

mod device_actor;
mod error;
mod lifecycle;
mod service;
mod utils;

pub use device_actor::DeviceActor;
pub use error::DeviceError;

pub use actor_framework::{ResourceActor, ResourceClient};
use capture_api::CaptureSender;
use netsim_model::chip::{ChipClient, NetworkKind};
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;
use std::sync::Arc;

/// Creates a new Device actor and its client.
pub fn new(
    chip_clients: HashMap<NetworkKind, Box<dyn ChipClient>>,
    next_chip_id: Arc<AtomicU32>,
    capture_client: Option<Arc<dyn CaptureSender>>,
) -> DeviceActor {
    DeviceActor {
        chip_clients,
        next_chip_id,
        capture_client,
        devices: HashMap::new(),
        next_device_id: 1,
    }
}
