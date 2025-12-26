// src/network_service.rs

use crate::{
    modem::ModemImpl,
    parser::Command,
    traits::CommandExecutor,
    types::{ExecutionResult, HandledCommand},
};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RegistrationStatus {
    NotRegistered,
    Registered,
}

// Holds all state related to the network.
pub struct NetworkService {
    operator_name: String,
    registration_status: RegistrationStatus,
}

impl NetworkService {
    /// Creates a new NetworkService.
    pub fn new() -> Self {
        Self {
            // This will be loaded from config later.
            operator_name: "Android Virtual Operator".to_string(),
            registration_status: RegistrationStatus::NotRegistered,
        }
    }

    pub fn handle_registration_complete(&mut self) -> ExecutionResult {
        self.registration_status = RegistrationStatus::Registered;
        ExecutionResult::Handled(HandledCommand {
            responses: vec!["+CREG: 1\r\n".to_string()],
            action: None,
        })
    }

    // --- Pure command handlers ---

    pub fn handle_query_operator(&self) -> ExecutionResult {
        let response = format!("+COPS: 0,0,\"{}\"\r\n", self.operator_name);
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    pub fn handle_query_signal_strength(&self) -> ExecutionResult {
        // For now, return a fixed value.
        let (rssi, ber) = (20, 99);
        let response = format!("+CSQ: {},{}\r\n", rssi, ber);
        let mut handled = HandledCommand::ok();
        handled.responses.insert(0, response);
        ExecutionResult::Handled(handled)
    }

    pub fn handle_query_extended_signal_quality(&self) -> ExecutionResult {
        // Not implemented yet, just return OK.
        ExecutionResult::Handled(HandledCommand::ok())
    }
}

impl CommandExecutor for NetworkService {
    fn execute(&self, _context: &ModemImpl, command: &Command) -> ExecutionResult {
        match command {
            Command::QueryOperator => self.handle_query_operator(),
            Command::QuerySignalStrength => self.handle_query_signal_strength(),
            Command::QueryExtendedSignalQuality => self.handle_query_extended_signal_quality(),
            _ => ExecutionResult::Unhandled,
        }
    }
}
