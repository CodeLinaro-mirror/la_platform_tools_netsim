use std::sync::{Arc, Mutex, Weak};

use crate::{
    call_service::CallService,
    data_service::DataService,
    misc_service::MiscService,
    modem_network_simulator::ModemNetworkSimulator,
    network_service::NetworkService,
    parser::Command,
    sim_service::SimService,
    sms_service::SmsService,
    stk_service::StkService,
    sup_service::SupService,
    traits::CommandExecutor,
    types::{Callbacks, CallbacksExt, CommandAction, ExecutionResult, ModemId},
};

/// Represents a single modem device.
pub struct ModemImpl {
    pub id: ModemId,
    pub(crate) callbacks: Arc<dyn Callbacks>,
    pub(crate) network: Weak<ModemNetworkSimulator>,
    pub sim_service: SimService,
    pub(crate) network_service: Mutex<NetworkService>,
    pub(crate) sms_service: SmsService,
    pub call_service: CallService,
    pub(crate) stk_service: StkService,
    pub(crate) sup_service: SupService,
    pub(crate) misc_service: MiscService,
    pub data_service: DataService,
    phone_number: Mutex<String>,
    state: Mutex<State>,
}

pub type Modem = Arc<ModemImpl>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Idle,
    WaitingForSmsPdu(usize, bool),
}

// An enum representing what to do for a scheduled event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModemEvent {
    CallRingTimeout { call_token: u32 },
    NetworkRegistrationComplete,
    // For testing purposes
    TestEvent,
}

impl ModemImpl {
    pub(crate) fn new(
        id: ModemId,
        callbacks: Arc<dyn Callbacks>,
        network: Weak<ModemNetworkSimulator>,
        profile: crate::config::SimProfile,
    ) -> Modem {
        let modem = Arc::new(Self {
            id,
            callbacks,
            network: network.clone(),
            sim_service: SimService::new(&profile),
            network_service: Mutex::new(NetworkService::new()),
            sms_service: SmsService::new(),
            stk_service: StkService::new(),
            sup_service: SupService::new(),
            misc_service: MiscService::new(),
            call_service: CallService::new(),
            data_service: DataService::new(),
            phone_number: Mutex::new("".to_string()),
            state: Mutex::new(State::Idle),
        });

        if let Some(network) = network.upgrade() {
            network.schedule_event(
                id,
                std::time::Duration::from_millis(10),
                ModemEvent::NetworkRegistrationComplete,
            );
        }

        modem
    }

    pub fn set_phone_number(&self, number: &str) {
        *self.phone_number.lock().unwrap() = number.to_string();
    }

    pub fn phone_number(&self) -> String {
        self.phone_number.lock().unwrap().clone()
    }

    pub fn set_waiting_for_sms_pdu(&self, len: usize, store: bool) {
        let mut state = self.state.lock().unwrap();
        *state = State::WaitingForSmsPdu(len, store);
    }

    /// Receives an AT command from the modem.
    pub fn receive_at_command(&self, command_bytes: &[u8]) {
        // Check for SMS PDU submission first. This requires special state handling.
        let sms_pdu_action = {
            let mut state = self.state.lock().unwrap();
            if let State::WaitingForSmsPdu(_, store) = *state {
                if command_bytes.ends_with(b"\x1a") {
                    let pdu = &command_bytes[..command_bytes.len() - 1];
                    let result = if store {
                        self.sms_service.handle_store_sms(self, pdu)
                    } else {
                        self.sms_service.handle_sms_body(self, pdu)
                    };
                    *state = State::Idle;
                    // Return the action to be handled outside the lock.
                    Some(result)
                } else {
                    None
                }
            } else {
                None
            }
        };

        if let Some(result) = sms_pdu_action {
            if let ExecutionResult::Handled(handled) = result {
                for response in handled.responses {
                    if !response.is_empty() {
                        self.callbacks.send_at_response(self.id, response.as_bytes());
                    }
                }
                if let Some(action) = handled.action {
                    self.handle_command_action(action);
                }
            }
            return;
        }

        // Proceed with normal command parsing.
        match Command::parse(command_bytes) {
            Ok((_, command)) => {
                let result = self.execute(&command);

                match result {
                    ExecutionResult::Handled(handled) => {
                        for response in handled.responses {
                            if !response.is_empty() {
                                self.callbacks.send_at_response(self.id, response.as_bytes());
                            }
                        }
                        if let Some(action) = handled.action {
                            self.handle_command_action(action);
                        }
                    }
                    ExecutionResult::Unhandled => {
                        // Now that all services are migrated, this is an error.
                        log::error!("Unhandled command: {:?}", command);
                        self.callbacks.send_error(self.id);
                    }
                }
            }
            Err(_) => {
                self.callbacks.send_error(self.id);
            }
        }
    }

    fn handle_command_action(&self, action: CommandAction) {
        log::debug!("[Modem {}] Preparing to handle action: {:?}", self.id, action);
        if action == CommandAction::None {
            log::trace!("[Modem {}] Action is None, skipping.", self.id);
            return;
        }

        if let Some(network) = self.network.upgrade() {
            log::debug!("[Modem {}] Delegating action to ModemNetworkSimulator.", self.id);
            network.handle_command_action(self.id, action);
        } else {
            log::warn!("[Modem {}] ModemNetworkSimulator is gone, cannot handle action.", self.id);
        }
    }

    /// Ticks the modem's event loop..
    pub fn tick(&self) {
        // To be implemented
    }

    // This is where we'll handle events dispatched from the manager
    pub fn handle_event(&self, event: ModemEvent) {
        match event {
            ModemEvent::TestEvent => {
                self.callbacks.send_at_response(self.id, b"TEST_EVENT_FIRED\r\n");
            }
            ModemEvent::CallRingTimeout { call_token } => {
                self.call_service.handle_ring_timeout(call_token);
            }
            ModemEvent::NetworkRegistrationComplete => {
                let result = self.network_service.lock().unwrap().handle_registration_complete();
                if let ExecutionResult::Handled(handled) = result {
                    for response in handled.responses {
                        if !response.is_empty() {
                            self.callbacks.send_at_response(self.id, response.as_bytes());
                        }
                    }
                }
            }
        }
    }

    pub fn call_service(&self) -> &CallService {
        &self.call_service
    }

    pub fn execute(&self, command: &Command) -> crate::types::ExecutionResult {
        // This is the new "Chain of Responsibility" entry point.
        let result = self.misc_service.execute(self, command);
        if !matches!(result, ExecutionResult::Unhandled) {
            return result;
        }

        let result = self.sms_service.execute(self, command);
        if !matches!(result, ExecutionResult::Unhandled) {
            return result;
        }

        let result = self.call_service.execute(self, command);
        if !matches!(result, ExecutionResult::Unhandled) {
            return result;
        }

        let result = self.data_service.execute(self, command);
        if !matches!(result, ExecutionResult::Unhandled) {
            return result;
        }

        let result = self.network_service.lock().unwrap().execute(self, command);
        if !matches!(result, ExecutionResult::Unhandled) {
            return result;
        }

        let result = self.sim_service.execute(self, command);
        if !matches!(result, ExecutionResult::Unhandled) {
            return result;
        }

        let result = self.stk_service.execute(self, command);
        if !matches!(result, ExecutionResult::Unhandled) {
            return result;
        }

        let result = self.sup_service.execute(self, command);
        if !matches!(result, ExecutionResult::Unhandled) {
            return result;
        }

        // If we get here, no service handled the command.
        ExecutionResult::Unhandled
    }
}
