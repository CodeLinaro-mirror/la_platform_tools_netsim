// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_model::{ModemAction, Quirks};

// ...
use crate::modem_network_simulator::NetworkEvent;
pub use crate::types::ModemError;
use crate::types::{ModemId, ModemInfo, ModemSink, PhoneNumber};

pub trait ModemNetworkInterface: Send + Sync {
    fn add_modem(
        &mut self,
        chip_id: ModemId,
        sink: ModemSink,
        sim_type: Option<i32>,
        sim_profile: Option<String>,
        quirks: Quirks,
    ) -> Result<(), ModemError>;
    fn remove_modem(&mut self, chip_id: ModemId) -> Result<(), ModemError>;
    fn send_data(&mut self, chip_id: ModemId, data: &[u8]) -> Result<(), ModemError>;
    fn get_modem_info(&self, chip_id: ModemId) -> Result<ModemInfo, ModemError>;
    fn on_timer(&mut self, chip_id: ModemId) -> Result<(), ModemError>;
    fn perform_action(&mut self, action: ModemAction) -> Result<Vec<NetworkEvent>, ModemError>;
}

use crate::modem_network_simulator::ModemNetworkSimulator;

impl ModemNetworkInterface for ModemNetworkSimulator {
    fn add_modem(
        &mut self,
        chip_id: ModemId,
        sink: ModemSink,
        sim_type: Option<i32>,
        sim_profile: Option<String>,
        quirks: Quirks,
    ) -> Result<(), ModemError> {
        self.new_modem(chip_id, sink, sim_type, sim_profile, quirks)
    }
    fn remove_modem(&mut self, chip_id: ModemId) -> Result<(), ModemError> {
        self.remove_modem(chip_id);
        Ok(())
    }

    fn send_data(&mut self, chip_id: ModemId, data: &[u8]) -> Result<(), ModemError> {
        self.send_at_command(chip_id, data);
        Ok(())
    }

    fn get_modem_info(&self, chip_id: ModemId) -> Result<ModemInfo, ModemError> {
        if let Some(modem) = self.get_modem(chip_id) {
            let (rssi, ber) = modem.network_service.signal_strength();
            let calls = modem
                .call_service
                .calls
                .iter()
                .map(|c| netsim_model::Call {
                    number: c
                        .number
                        .as_ref()
                        .map(|n: &PhoneNumber| n.as_str().to_string())
                        .unwrap_or_default(),
                    state: match c.state {
                        crate::call_service::CallState::Active => netsim_model::CallState::Active,
                        crate::call_service::CallState::Held => netsim_model::CallState::Holding,
                        crate::call_service::CallState::Dialing => netsim_model::CallState::Dialing,
                        crate::call_service::CallState::Alerting => {
                            netsim_model::CallState::Alerting
                        }
                        crate::call_service::CallState::Incoming => {
                            netsim_model::CallState::Incoming
                        }
                        crate::call_service::CallState::Waiting => netsim_model::CallState::Waiting,
                    },
                    direction: match c.direction {
                        crate::call_service::CallDirection::Outgoing => {
                            netsim_model::CallDirection::MobileOriginated
                        }
                        crate::call_service::CallDirection::Incoming => {
                            netsim_model::CallDirection::MobileTerminated
                        }
                    },
                })
                .collect();
            Ok(ModemInfo {
                id: chip_id,
                calls,
                ringing: modem.is_ringing(),
                sms_count: modem.get_sms_count(),
                quirks: modem.quirks,
                rssi: rssi as u32,
                ber: ber as u32,
                voice_registration: modem.network_service.voice_registration(),
                data_registration: modem.network_service.data_registration(),
            })
        } else {
            Err(ModemError::NotFound)
        }
    }

    fn on_timer(&mut self, chip_id: ModemId) -> Result<(), ModemError> {
        self.on_timer(chip_id);
        Ok(())
    }

    fn perform_action(&mut self, action: ModemAction) -> Result<Vec<NetworkEvent>, ModemError> {
        Ok(self.dispatch(action))
    }
}
