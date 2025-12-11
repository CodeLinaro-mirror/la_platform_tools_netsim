// Copyright 2025 The Android Open Source Project

use crate::context::BluetoothContext;
use crate::entity::BluetoothEntity;
use crate::error::BluetoothError;
use crate::handlers::events::HciCallbacks;
use crate::utils::ToChipError;
use actor_framework::Runtime;
use netsim_model::chip::{BluetoothMode, ChipId, ChipUpdate};
use netsim_model::chip_error::ChipError;

// TODO: don't split these functions across files, see if they can be
// implemented along with BluetoothEntity, possibly with util functions
// to keep the size down.

pub async fn on_create(
    entity: &mut BluetoothEntity,
    context: &mut BluetoothContext,
    runtime: &mut impl Runtime,
) -> Result<(), BluetoothError> {
    let id = ChipId(entity.chip.id);

    // 1. Register Stream
    if let Some(stream) = entity.packet_stream.lock().unwrap().take() {
        runtime.add_stream(id.0 as usize, Box::pin(stream));
    }

    // 2. Setup Sink and Callbacks
    let callback = if let Some(sink) = entity.packet_sink.lock().unwrap().take() {
        // Create a channel to send HCI packets from the callback to the sink task.
        let (hci_tx, hci_rx) = tokio::sync::mpsc::channel(10);

        // Spawn the sink task which forwards packets from the channel to the sink.
        let sink_id = id;
        runtime.add_task(
            sink_id.0 as usize,
            Box::pin(async move {
                run_sink_task(sink, hci_rx, sink_id).await;
            }),
        );

        HciCallbacks { id, hci_tx: Some(hci_tx), ll_tx: None }
    } else {
        HciCallbacks { id, hci_tx: None, ll_tx: None }
    };

    // 3. Create Rootcanal Controller
    let create_params = entity.create_params.take().ok_or(BluetoothError::Chip(
        ChipError::InvalidArguments("Missing Bluetooth params".into()),
    ))?;

    let address =
        create_params.address.parse().unwrap_or_else(|_| "00:00:00:00:00:00".parse().unwrap());

    context.rootcanal.new_controller(id.0.into(), address, Box::new(callback)).to_chip_error()?;

    // 4. Create Chip Info in Context
    // Initialize the chip info based on the mode (Beacon, Device, or Sniffer).

    let mut chip_info = match &create_params.mode {
        BluetoothMode::Beacon(params) => crate::beacon::create(&context.rootcanal, id, params)?,
        BluetoothMode::Device(params) => crate::device::create(&context.rootcanal, id, params)?,
        BluetoothMode::Sniffer(params) => crate::sniffer::create(&context.rootcanal, id, params)?,
    };
    chip_info.device_id = entity.chip.device_id;
    context.chips.lock().unwrap().insert(id, chip_info);

    Ok(())
}

async fn run_sink_task(
    mut sink: netsim_model::chip::PacketSink,
    mut receiver: tokio::sync::mpsc::Receiver<bytes::Bytes>,
    id: ChipId,
) {
    use futures::SinkExt;
    while let Some(packet) = receiver.recv().await {
        if sink.send(packet).await.is_err() {
            log::info!("Sink for chip {} failed", id);
            break;
        }
    }
    log::info!("Sink task for chip {} finished", id);
}

pub async fn on_update(
    entity: &mut BluetoothEntity,
    update: ChipUpdate,
    context: &mut BluetoothContext,
    _runtime: &mut impl Runtime,
) -> Result<(), BluetoothError> {
    let mut chips = context.chips.lock().unwrap();
    if let Some(chip) = chips.get_mut(&ChipId(entity.chip.id)) {
        if let Some(pos) = update.position {
            chip.position = pos.clone();
            entity.chip.position = pos;
        }
        if let Some(orient) = update.orientation {
            chip.orientation = orient.clone();
            entity.chip.orientation = orient;
        }
        // TODO: Handle other fields
    }
    Ok(())
}

pub async fn on_delete(
    entity: &BluetoothEntity,
    context: &mut BluetoothContext,
    _runtime: &mut impl Runtime,
) -> Result<(), BluetoothError> {
    let id = ChipId(entity.chip.id);
    log::info!("Deleting chip {id}");
    context.chips.lock().unwrap().remove(&id);
    context.rootcanal.remove_controller(id.0.into()).to_chip_error()?;

    // Notify DeviceService
    let dc = context.device_client.clone();
    let device_id = entity.chip.device_id;
    tokio::spawn(async move {
        let _ = dc.notify_chip_removed(device_id, id).await;
    });

    Ok(())
}
