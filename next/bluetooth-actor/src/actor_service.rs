// Copyright 2025 The Android Open Source Project

use actor_framework::{ActorService, DynContext};
use netsim_model::{
    chip::{
        BluetoothMode, Chip, ChipCreate, ChipKindParams, ChipUpdate, ChipVariant, ChipVariantUpdate,
    },
    chip_error::ChipError,
    ChipId, ChipKind,
};
use tracing::{info, warn};

use crate::{
    actions::{BluetoothAction, BluetoothActionResult},
    bluetooth_actor::BluetoothActor,
    error::BluetoothError,
    hci_callbacks::HciCallbacks,
    utils::ToChipError,
};

impl ActorService for BluetoothActor {
    type Id = ChipId;
    type Create = ChipCreate;
    type Update = ChipUpdate;
    type Action = BluetoothAction;
    type ActionResult = BluetoothActionResult;
    type Error = BluetoothError;
    type Entity = Chip;
    type TypedStream = ();

    async fn handle_create(
        &mut self,
        id: Option<Self::Id>,
        params: Self::Create,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Self::Id, Self::Error> {
        let chip_id = id.ok_or_else(|| BluetoothError::invalid_arg("missing chip id"))?;

        let config = &params.config;
        let create_params = match &config.chip_kind_params {
            ChipKindParams::Bluetooth(p) => p,
            _ => {
                return Err(BluetoothError::invalid_arg("Expected Bluetooth network params"));
            }
        };

        // Validate Scanner constraints: No PacketStream, Must have PacketSink.
        if let BluetoothMode::Scanner(_) = &create_params.mode {
            assert!(params.packet_stream.is_none(), "Scanner chip cannot have a packet stream");
            assert!(params.packet_sink.is_some(), "Scanner chip must have a packet sink");
        }

        // Validate Sniffer constraints: No PacketStream, Must have PacketSink.
        if let BluetoothMode::Sniffer(_) = &create_params.mode {
            assert!(params.packet_stream.is_none(), "Sniffer chip cannot have a packet stream");
            assert!(params.packet_sink.is_some(), "Sniffer chip must have a packet sink");
        }

        let name =
            if config.name.is_empty() && matches!(create_params.mode, BluetoothMode::Beacon(_)) {
                let name = crate::beacon_utils::generate_default_name(chip_id.0);
                self.sync_device_name(params.device_id, name.clone());
                name
            } else {
                config.name.clone()
            };

        let raw_address = if create_params.address.is_empty() {
            crate::beacon_utils::generate_legacy_address(chip_id.into())
        } else {
            create_params.address.clone()
        };

        let address: rootcanal::Address =
            raw_address.parse().map_err(|_| BluetoothError::invalid_arg("Invalid address"))?;

        let mut mode = create_params.mode.clone();
        if let BluetoothMode::Beacon(ref mut beacon_params) = mode {
            if beacon_params.ble_beacon.address.is_empty() {
                beacon_params.ble_beacon.address = raw_address;
            }
        }

        let chip = Chip {
            id: chip_id.0,
            device_id: params.device_id,
            name,
            manufacturer: params.config.manufacturer.clone(),
            product_name: params.config.product_name.clone(),
            kind: ChipKind::BLUETOOTH,
            variant: Some(ChipVariant::Bluetooth(Default::default())),
            ..Default::default()
        };

        // 1. Register Stream
        if let Some(stream) = params.packet_stream {
            _ctx.add_stream(chip_id, Box::pin(stream));
        }

        // 2. Setup Sink and Callbacks
        let callback = if let Some(sink) = params.packet_sink {
            // Bounded to 100 packets (~100ms of lag absorption at 2Mbps) to:
            // - Tolerate transient guest lag.
            // - Protect netsimd from OOM crashes if the client becomes unresponsive.
            let (tx, rx) = tokio::sync::mpsc::channel(100);

            // Spawn the sink task which forwards packets from the channel to the sink.
            let sink_id = chip_id;
            _ctx.spawn(sink_id, Box::pin(crate::hci_callbacks::sink_loop(sink, rx, sink_id)));

            if let BluetoothMode::Sniffer(_) = &mode {
                HciCallbacks { id: chip_id, hci_tx: None, ll_tx: Some(tx) }
            } else {
                HciCallbacks { id: chip_id, hci_tx: Some(tx), ll_tx: None }
            }
        } else {
            HciCallbacks { id: chip_id, hci_tx: None, ll_tx: None }
        };

        // 3. Create Rootcanal Controller
        // Construct the Protobuf configuration for Rootcanal
        let mut quirks = netsim_proto::configuration::ControllerQuirks::new();
        // This quirk forces Rootcanal to reject post-init commands from uninitialized
        // hosts, causing a HAL restart that properly unmasks the LE Meta
        // events. We only apply this to actual Emulator Devices, not internal
        // netsim beacons or scanners.
        if let BluetoothMode::Device(_) = &mode {
            quirks.hardware_error_before_reset = Some(true);
        }

        let mut config_controller = netsim_proto::configuration::Controller::new();
        config_controller.quirks = netsim_proto::protobuf::MessageField::some(quirks);

        let config_bytes = netsim_proto::protobuf::Message::write_to_bytes(&config_controller)
            .map_err(|e| {
                BluetoothError::invalid_arg(format!("Failed to serialize bt config: {e}"))
            })?;

        self.rootcanal
            .new_controller(chip_id.0.into(), address, Box::new(callback), Some(&config_bytes))
            .to_chip_error()?;

        // 4. Create Chip Info in Context
        let mut chip_info = match &mode {
            BluetoothMode::Beacon(params) => {
                crate::beacon::create(&self.rootcanal, chip_id, params, &chip.name)?
            }
            BluetoothMode::Device(params) => {
                crate::device::create(&self.rootcanal, chip_id, params)?
            }
            BluetoothMode::Scanner(params) => {
                crate::scanner::create(&self.rootcanal, chip_id, params)?
            }
            BluetoothMode::Sniffer(params) => {
                crate::sniffer::create(&self.rootcanal, chip_id, params)?
            }
        };
        chip_info.device_id = chip.device_id;
        self.chips.lock().unwrap().insert(id.unwrap_or(chip_id), chip);
        Ok(chip_id)
    }

    async fn handle_get(
        &self,
        id: Self::Id,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Option<Self::Entity>, Self::Error> {
        let chips = self.chips.lock().unwrap();
        Ok(chips.get(&id).cloned())
    }

    // TODO: Implement radio state enforcement (stopping HCI/transmission when
    // disabled).
    async fn handle_update(
        &mut self,
        id: Self::Id,
        update: Self::Update,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Self::Entity, Self::Error> {
        let mut chips = self.chips.lock().unwrap();
        let mut chip =
            chips.get(&id).cloned().ok_or(BluetoothError::Chip(ChipError::ChipNotFound(id)))?;

        // 1. Update the chip data first
        if let Some(pos) = update.position {
            chip.position = pos;
        }
        if let Some(orient) = update.orientation {
            chip.orientation = orient;
        }
        if let Some(links) = update.links {
            chip.links = links;
        }
        if let Some(enabled) = update.enabled {
            chip.enabled = enabled;
        }

        // 2. Handle Variant logic
        if let Some(ChipVariantUpdate::Bluetooth(bt_update)) = update.variant {
            if let Some(ChipVariant::Bluetooth(bt_chip)) = &mut chip.variant {
                bt_update.classic.apply(&mut bt_chip.classic);
                bt_update.low_energy.apply(&mut bt_chip.low_energy);
            }
        }

        // 3. Sync the global chips map
        chips.insert(id, chip.clone());

        Ok(chip)
    }

    async fn handle_delete(
        &mut self,
        id: Self::Id,
        _ctx: &mut DynContext<Self>,
    ) -> Result<(), Self::Error> {
        let mut chips = self.chips.lock().unwrap();
        if let Some(chip) = chips.remove(&id) {
            let chip_id = ChipId(chip.id);
            let device_id = chip.device_id;

            info!("Deleting chip {chip_id}");
            self.rootcanal.remove_controller(chip_id.0.into()).to_chip_error()?;

            // Notify DeviceService
            let dc = self.device_client.clone();
            if device_id.0 != 0 {
                tokio::spawn(async move {
                    let _ = dc.notify_chip_removed(device_id, chip_id).await;
                });
            }
            Ok(())
        } else {
            Err(BluetoothError::Chip(ChipError::ChipNotFound(id)))
        }
    }

    async fn handle_action(
        &mut self,
        _id: Option<Self::Id>,
        _action: Self::Action,
        _ctx: &mut DynContext<Self>,
    ) -> Result<Self::ActionResult, Self::Error> {
        match _action {
            BluetoothAction::Reset { id } => {
                info!("Resetting Bluetooth chip {id}");
                let _ = self.rootcanal.clear_stats(id.0.into());

                let mut chips = self.chips.lock().unwrap();
                if let Some(chip) = chips.get_mut(&id) {
                    chip.enabled = true;
                    if let Some(ChipVariant::Bluetooth(bt)) = &mut chip.variant {
                        bt.low_energy.state = Some(true);
                        bt.classic.state = Some(true);
                    }
                }
                Ok(BluetoothActionResult::Success)
            }
            BluetoothAction::GetStatistics => {
                let mut stats_list = Vec::new();
                let chips = self.chips.lock().unwrap();
                for (id, chip) in chips.iter() {
                    if let Ok(stats) = self.rootcanal.get_stats(id.0.into()) {
                        // BLE Stats
                        stats_list.push(netsim_model::stats::NetsimRadioStats {
                            id: id.0,
                            name: chip.name.clone(),
                            kind: netsim_model::stats::RadioKind::BluetoothLowEnergy,
                            tx_count: stats.ll_packets_out_ble,
                            rx_count: stats.ll_packets_in_ble,
                            tx_bytes: 0,
                            rx_bytes: 0,
                            ..Default::default()
                        });

                        // Classic Stats
                        stats_list.push(netsim_model::stats::NetsimRadioStats {
                            id: id.0,
                            name: chip.name.clone(),
                            kind: netsim_model::stats::RadioKind::BluetoothClassic,
                            tx_count: stats.ll_packets_out_classic,
                            rx_count: stats.ll_packets_in_classic,
                            tx_bytes: 0,
                            rx_bytes: 0,
                            ..Default::default()
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
        _ctx: &mut DynContext<Self>,
    ) -> Result<Vec<Self::Entity>, Self::Error> {
        let chips = self.chips.lock().unwrap();
        Ok(chips.values().cloned().collect())
    }
}

impl BluetoothActor {
    /// Asynchronously synchronize the device name to match the chip name.
    fn sync_device_name(&self, device_id: netsim_model::device::DeviceId, name: String) {
        let dc = self.device_client.clone();
        tokio::spawn(async move {
            let mut update = netsim_model::device::api::DeviceUpdate::default();
            update.name = Some(name);
            if let Err(e) = dc.update(device_id, update).await {
                warn!("Failed to sync device name for device {}: {:?}", device_id, e);
            }
        });
    }
}
