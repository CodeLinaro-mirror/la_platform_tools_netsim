// Copyright 2025 The Android Open Source Project

use crate::actions::{BluetoothAction, BluetoothActionResult};
use crate::bluetooth_actor::BluetoothActor;
use crate::error::BluetoothError;
use crate::hci_callbacks::HciCallbacks;
use crate::utils::ToChipError;
use actor_framework::{ActorService, DynContext};
use async_trait::async_trait;
use netsim_model::chip::{BluetoothMode, Chip, ChipCreate, ChipId, ChipUpdate};
use netsim_model::chip_error::ChipError;

use crate::internal_chip::InternalChip;

#[async_trait]
impl ActorService for BluetoothActor {
    type Id = ChipId;
    type Create = ChipCreate;
    type Update = ChipUpdate;
    type Action = BluetoothAction;
    type ActionResult = BluetoothActionResult;
    type Error = BluetoothError;
    type Entity = Chip;

    async fn handle_create(
        &mut self,
        id: Option<Self::Id>,
        params: Self::Create,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::Id, Self::Error> {
        let id = params.id;
        let mut entity = InternalChip::from_create_params(id, params)?;

        let chip_id = ChipId(entity.chip.id);

        // 1. Register Stream
        if let Some(stream) = entity.packet_stream.take() {
            _ctx.add_stream(chip_id, Box::pin(stream));
        }

        // 2. Setup Sink and Callbacks
        let callback = if let Some(sink) = entity.packet_sink.take() {
            // Create a channel to send HCI packets from the callback to the sink task.
            let (hci_tx, hci_rx) = tokio::sync::mpsc::channel(10);

            // Spawn the sink task which forwards packets from the channel to the sink.
            let sink_id = chip_id;
            _ctx.spawn(sink_id, Box::pin(crate::hci_callbacks::sink_loop(sink, hci_rx, sink_id)));

            HciCallbacks { id: chip_id, hci_tx: Some(hci_tx), ll_tx: None }
        } else {
            HciCallbacks { id: chip_id, hci_tx: None, ll_tx: None }
        };

        // 3. Create Rootcanal Controller
        let create_params = entity.create_params.take().ok_or(BluetoothError::Chip(
            ChipError::InvalidArguments("Missing Bluetooth params".into()),
        ))?;

        let address = create_params.address.parse().map_err(|_| {
            BluetoothError::Chip(ChipError::InvalidArguments("Invalid address".into()))
        })?;
        // Note: There is no specific enforcement for a "blue" address type.
        // The current check only validates if the address string is parsable.

        self.rootcanal
            .new_controller(chip_id.0.into(), address, Box::new(callback))
            .to_chip_error()?;

        // 4. Create Chip Info in Context
        // Initialize the chip info based on the mode (Beacon, Device, or Sniffer).

        let mut chip_info = match &create_params.mode {
            BluetoothMode::Beacon(params) => {
                crate::beacon::create(&self.rootcanal, chip_id, params)?
            }
            BluetoothMode::Device(params) => {
                crate::device::create(&self.rootcanal, chip_id, params)?
            }
            BluetoothMode::Sniffer(params) => {
                crate::sniffer::create(&self.rootcanal, chip_id, params)?
            }
        };
        chip_info.device_id = entity.chip.device_id;
        self.chips.lock().unwrap().insert(chip_id, chip_info);
        self.chips.lock().unwrap().insert(id, entity.chip.clone());
        self.entities.insert(id, entity);
        Ok(id)
    }

    async fn handle_get(
        &self,
        id: Self::Id,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Option<Self::Entity>, Self::Error> {
        Ok(self.entities.get(&id).map(|e| e.chip.clone()))
    }

    async fn handle_update(
        &mut self,
        id: Self::Id,
        update: Self::Update,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::Entity, Self::Error> {
        if let Some(mut entity) = self.entities.remove(&id) {
            // Lock chips once and reuse the guard to avoid deadlock.
            let mut chips = self.chips.lock().unwrap();
            if let Some(chip) = chips.get_mut(&ChipId(entity.chip.id)) {
                if let Some(pos) = update.position {
                    chip.position = pos.clone();
                    entity.chip.position = pos;
                }
                if let Some(orient) = update.orientation {
                    chip.orientation = orient.clone();
                    entity.chip.orientation = orient;
                }
                if let Some(links) = update.links {
                    chip.links = links.clone();
                    entity.chip.links = links;
                }
                // TODO: Handle other fields
            }

            chips.insert(id, entity.chip.clone());
            self.entities.insert(id, entity);
            Ok(chips.get(&id).unwrap().clone())
        } else {
            Err(BluetoothError::Chip(ChipError::ChipNotFound(id)))
        }
    }

    async fn handle_delete(
        &mut self,
        id: Self::Id,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<(), Self::Error> {
        if let Some(entity) = self.entities.remove(&id) {
            let chip_id = ChipId(entity.chip.id);
            log::info!("Deleting chip {chip_id}");
            self.chips.lock().unwrap().remove(&chip_id);
            self.rootcanal.remove_controller(chip_id.0.into()).to_chip_error()?;

            // Notify DeviceService
            let dc = self.device_client.clone();
            let device_id = entity.chip.device_id;
            tokio::spawn(async move {
                let _ = dc.notify_chip_removed(device_id, chip_id).await;
            });
            self.chips.lock().unwrap().remove(&id);
            Ok(())
        } else {
            Err(BluetoothError::Chip(ChipError::ChipNotFound(id)))
        }
    }

    async fn handle_action(
        &mut self,
        _id: Option<Self::Id>,
        _action: Self::Action,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Self::ActionResult, Self::Error> {
        match _action {
            BluetoothAction::Reset { id } => {
                // TODO: Implement reset
                log::warn!("Reset chip {id} not implemented");
                let _ = self.rootcanal.clear_stats(id.0.into());
                Ok(BluetoothActionResult::Success)
            }
            BluetoothAction::GetStatistics => {
                let mut stats_list = Vec::new();
                let chips = self.chips.lock().unwrap();
                for (id, chip) in chips.iter() {
                    if let Ok(stats) = self.rootcanal.get_stats(id.0.into()) {
                        stats_list.push(netsim_model::stats::NetsimRadioStats {
                            id: id.0,
                            name: chip.name.clone().unwrap_or("Unknown".to_string()),
                            tx_bytes: stats.ll_packets_out,
                            rx_bytes: stats.ll_packets_in,
                        });
                    }
                }
                Ok(BluetoothActionResult::Statistics(stats_list.into_boxed_slice()))
            }
            BluetoothAction::GetCountForTesting => {
                let count = self.chips.lock().unwrap().len();
                Ok(BluetoothActionResult::Count(count))
            }
        }
    }

    async fn handle_list(
        &mut self,
        _ctx: &mut DynContext<Self::Id>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        Ok(actor_framework::utils::handle_list_map(&self.entities, |e| e.chip.clone()))
    }
}
