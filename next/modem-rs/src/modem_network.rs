use netsim_model::cell::ModemAction;

// ...
use crate::modem_network_simulator::NetworkEvent;
pub use crate::types::ModemError;
use crate::types::{ModemId, ModemInfo, ModemSink};

pub trait ModemNetworkInterface: Send + Sync {
    fn add_modem(&mut self, chip_id: ModemId, sink: ModemSink) -> Result<(), ModemError>;
    fn remove_modem(&mut self, chip_id: ModemId) -> Result<(), ModemError>;
    fn send_data(&mut self, chip_id: ModemId, data: &[u8]) -> Result<(), ModemError>;
    fn get_modem_info(&self, chip_id: ModemId) -> Result<ModemInfo, ModemError>;
    fn on_timer(&mut self, chip_id: ModemId) -> Result<(), ModemError>;
    fn perform_action(&mut self, action: ModemAction) -> Result<Vec<NetworkEvent>, ModemError>;
}

use crate::modem_network_simulator::ModemNetworkSimulator;

impl ModemNetworkInterface for ModemNetworkSimulator {
    fn add_modem(&mut self, chip_id: ModemId, sink: ModemSink) -> Result<(), ModemError> {
        self.new_modem(chip_id, sink)
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
            Ok(ModemInfo {
                id: chip_id,
                connections: modem.get_active_calls(),
                ringing: modem.is_ringing(),
                sms_count: modem.get_sms_count(),
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
