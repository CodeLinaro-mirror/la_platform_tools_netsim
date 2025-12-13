// Copyright 2025 The Android Open Source Project

use crate::actions::{BluetoothAction, BluetoothActionResult};
use crate::actor::BluetoothActor;
use crate::error::BluetoothError;
use crate::handlers::actions::handle_action;
use crate::handlers::lifecycle::{on_create, on_delete, on_update};
use actor_framework::{ActorService, Context};
use async_trait::async_trait;
use netsim_model::chip::{
    BluetoothCreate, Chip, ChipCreate, ChipId, ChipUpdate, NetworkParams, PacketSink, PacketStream,
};
use netsim_model::chip_error::ChipError;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
/// The entity representing a Bluetooth chip.
pub struct BluetoothEntity {
    /// The underlying chip state.
    pub chip: Chip,
    /// Temporary storage for the packet stream, moved to runtime in `on_create`.
    pub packet_stream: Arc<Mutex<Option<PacketStream>>>,
    /// Temporary storage for the packet sink, moved to a task in `on_create`.
    pub packet_sink: Arc<Mutex<Option<PacketSink>>>,
    /// Creation parameters preserved for debugging or restart.
    pub create_params: Option<BluetoothCreate>,
}

impl std::fmt::Debug for BluetoothEntity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BluetoothEntity")
            .field("chip", &self.chip)
            .field("packet_stream", &"PacketStream")
            .field("packet_sink", &"PacketSink")
            .field("create_params", &self.create_params)
            .finish()
    }
}

#[async_trait]
impl ActorService for BluetoothEntity {
    type Id = ChipId;
    type Create = ChipCreate;
    type Update = ChipUpdate;
    type Action = BluetoothAction;
    type ActionResult = BluetoothActionResult;
    type Context = BluetoothActor;
    type Error = BluetoothError;
    type ListResponse = Vec<Chip>;

    fn from_create_params(id: Self::Id, mut params: Self::Create) -> Result<Self, Self::Error> {
        let chip = Chip {
            id: id.0,
            device_id: params.device_id,
            name: Some(params.config.name),
            manufacturer: Some(params.config.manufacturer),
            product_name: Some(params.config.product_name),
            kind: netsim_model::chip::ChipKind::BLUETOOTH,
            ..Default::default()
        };

        let create_params = match params.config.network_params {
            NetworkParams::Bluetooth(p) => Some(p),
            _ => {
                return Err(BluetoothError::Chip(ChipError::InvalidArguments(
                    "Expected Bluetooth network params".into(),
                )));
            }
        };

        Ok(Self {
            chip,
            packet_stream: Arc::new(Mutex::new(params.packet_stream.take())),
            packet_sink: Arc::new(Mutex::new(params.packet_sink.take())),
            create_params,
        })
    }

    async fn on_create(
        &mut self,
        actor: &mut Self::Context,
        ctx: &mut impl Context,
    ) -> Result<(), Self::Error> {
        on_create(self, actor, ctx).await
    }

    async fn on_update(
        &mut self,
        update: Self::Update,
        actor: &mut Self::Context,
        ctx: &mut impl Context,
    ) -> Result<(), Self::Error> {
        on_update(self, update, actor, ctx).await
    }

    async fn on_delete(
        &self,
        actor: &mut Self::Context,
        ctx: &mut impl Context,
    ) -> Result<(), Self::Error> {
        on_delete(self, actor, ctx).await
    }

    async fn handle_action(
        &mut self,
        action: Self::Action,
        actor: &mut Self::Context,
        ctx: &mut impl Context,
    ) -> Result<Self::ActionResult, Self::Error> {
        handle_action(self, action, actor, ctx).await
    }

    fn on_list(
        entities: &HashMap<Self::Id, Self>,
        _context: &mut Self::Context,
        _ctx: &mut impl Context,
    ) -> Self::ListResponse {
        entities.values().map(|e| e.chip.clone()).collect()
    }
}
