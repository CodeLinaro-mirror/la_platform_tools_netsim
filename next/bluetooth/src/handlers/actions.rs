// Copyright 2025 The Android Open Source Project

use crate::actions::{BluetoothAction, BluetoothActionResult};
use crate::context::BluetoothContext;
use crate::entity::BluetoothEntity;
use crate::error::BluetoothError;
use actor_framework::Runtime;

pub async fn handle_action(
    entity: &mut BluetoothEntity,
    action: BluetoothAction,
    context: &mut BluetoothContext,
    _runtime: &mut impl Runtime,
) -> Result<BluetoothActionResult, BluetoothError> {
    match action {
        BluetoothAction::Reset { id: _ } => {
            // TODO: Implement reset
            Ok(BluetoothActionResult::Success)
        }
        BluetoothAction::GetStatistics => {
            let mut stats_list = Vec::new();
            let chips = context.chips.lock().unwrap();
            for (id, chip) in chips.iter() {
                if let Ok(stats) = context.rootcanal.get_stats(id.0.into()) {
                    stats_list.push(netsim_model::stats::NetsimRadioStats {
                        id: id.0,
                        name: chip.name.clone().unwrap_or("Unknown".to_string()),
                        tx_bytes: stats.ll_packets_out,
                        rx_bytes: stats.ll_packets_in,
                    });
                }
            }
            Ok(BluetoothActionResult::Statistics(stats_list))
        }
        BluetoothAction::GetCountForTesting => {
            let count = context.chips.lock().unwrap().len();
            Ok(BluetoothActionResult::Count(count))
        }
    }
}
