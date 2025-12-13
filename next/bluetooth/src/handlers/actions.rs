// Copyright 2025 The Android Open Source Project

use crate::actions::{BluetoothAction, BluetoothActionResult};
use crate::actor::BluetoothActor;
use crate::error::BluetoothError;
use crate::service::BluetoothEntity;
use actor_framework::Context;

pub async fn handle_action(
    _entity: &mut BluetoothEntity,
    action: BluetoothAction,
    actor: &mut BluetoothActor,
    _ctx: &mut impl Context,
) -> Result<BluetoothActionResult, BluetoothError> {
    match action {
        BluetoothAction::Reset { id: _ } => {
            // TODO: Implement reset
            Ok(BluetoothActionResult::Success)
        }
        BluetoothAction::GetStatistics => {
            let mut stats_list = Vec::new();
            let chips = actor.chips.lock().unwrap();
            for (id, chip) in chips.iter() {
                if let Ok(stats) = actor.rootcanal.get_stats(id.0.into()) {
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
            let count = actor.chips.lock().unwrap().len();
            Ok(BluetoothActionResult::Count(count))
        }
    }
}
