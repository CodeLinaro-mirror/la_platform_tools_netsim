// Copyright 2025 The Android Open Source Project

use crate::error::BluetoothError;
use netsim_model::chip::{
    BluetoothCreate, Chip, ChipCreate, ChipId, ChipKind, NetworkParams, PacketSink, PacketStream,
};
use netsim_model::chip_error::ChipError;

/// The entity representing a Bluetooth chip.
pub(crate) struct InternalChip {
    /// The underlying chip state.
    pub chip: Chip,
    /// Temporary storage for the packet stream, moved to runtime in `on_create`.
    pub packet_stream: Option<PacketStream>,
    /// Temporary storage for the packet sink, moved to a task in `on_create`.
    pub packet_sink: Option<PacketSink>,
    /// Creation parameters preserved for debugging or restart.
    pub create_params: Option<BluetoothCreate>,
}

impl std::fmt::Debug for InternalChip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InternalChip")
            .field("chip", &self.chip)
            .field("packet_stream", &"PacketStream")
            .field("packet_sink", &"PacketSink")
            .field("create_params", &self.create_params)
            .finish()
    }
}

impl InternalChip {
    pub(crate) fn from_create_params(
        id: ChipId,
        mut params: ChipCreate,
    ) -> Result<Self, BluetoothError> {
        let chip = Chip {
            id: id.0,
            device_id: params.device_id,
            name: Some(params.config.name),
            manufacturer: Some(params.config.manufacturer),
            product_name: Some(params.config.product_name),
            kind: ChipKind::BLUETOOTH,
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
            packet_stream: params.packet_stream.take(),
            packet_sink: params.packet_sink.take(),
            create_params,
        })
    }
}
