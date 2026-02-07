// src/misc_service.rs

use std::sync::Mutex;

use crate::{
    modem::ModemImpl,
    parser::{Command, QuotedString},
    traits::CommandExecutor,
    types::{ExecutionResult, HandledCommand},
};

pub struct MiscService {
    clock: Mutex<String>,
}

impl MiscService {
    pub fn new() -> Self {
        Self { clock: Mutex::new("".to_string()) }
    }

    // --- Pure command handlers ---

    pub fn handle_set_time(&self, time: QuotedString) -> ExecutionResult {
        let mut clock = self.clock.lock().unwrap();
        *clock = String::from_utf8(time.to_vec()).unwrap_or_default();
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_query_time(&self) -> ExecutionResult {
        let clock = self.clock.lock().unwrap();
        let response_data = format!("+CCLK: \"{}\"\r\n", *clock);
        ExecutionResult::Handled(HandledCommand {
            responses: vec![response_data, "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_get_model_id(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand {
            responses: vec!["gLinux\r\n".to_string(), "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_get_revision(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand {
            responses: vec!["1.0\r\n".to_string(), "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_get_serial_number(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand {
            responses: vec!["0123456789\r\n".to_string(), "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_set_icf(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_ifc(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_ipr(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_report_mobile_equipment_error(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_echo(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_speaker_volume(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_speaker_mute(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_quiet_mode(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_verbose_mode(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_reset_to_factory_defaults(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_view_active_configuration(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_write_active_configuration(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_reset(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_get_identification_information(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_auto_answer(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_command_termination_character(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_response_formatting_character(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_command_line_editing_character(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_pause_before_blind_dialing(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_connection_completion_timeout(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_comma_dial_modifier_time(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_automatic_disconnect_delay(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_get_manufacturer_identification(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand {
            responses: vec!["Android\r\n".to_string(), "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_get_capabilities(&self) -> ExecutionResult {
        ExecutionResult::Handled(HandledCommand {
            responses: vec!["+GCAP: +FCLASS,+DS\r\nOK\r\n".to_string()],
            action: None,
        })
    }
}

impl CommandExecutor for MiscService {
    fn execute(&self, _context: &ModemImpl, command: &Command) -> ExecutionResult {
        match command {
            Command::GetManufacturerIdentification => self.handle_get_manufacturer_identification(),
            Command::GetCapabilities => self.handle_get_capabilities(),
            Command::GetModelId => self.handle_get_model_id(),
            Command::GetRevision => self.handle_get_revision(),
            Command::GetSerialNumber => self.handle_get_serial_number(),
            Command::SetTeTaControlCharacterFraming(_, _) => self.handle_set_icf(),
            Command::SetTeTaLocalDataFlowControl(_, _) => self.handle_set_ifc(),
            Command::SetTeTaFixedLocalRate(_) => self.handle_set_ipr(),
            Command::SetTime(time) => self.handle_set_time(*time),
            Command::QueryTime => self.handle_query_time(),
            Command::SetReportMobileEquipmentError(_) => {
                self.handle_set_report_mobile_equipment_error()
            }
            Command::SetEcho(_) => self.handle_set_echo(),
            Command::SetSpeakerVolume(_) => self.handle_set_speaker_volume(),
            Command::SetSpeakerMute(_) => self.handle_set_speaker_mute(),
            Command::SetQuietMode(_) => self.handle_set_quiet_mode(),
            Command::SetVerboseMode(_) => self.handle_set_verbose_mode(),
            Command::ResetToFactoryDefaults => self.handle_reset_to_factory_defaults(),
            Command::ViewActiveConfiguration => self.handle_view_active_configuration(),
            Command::WriteActiveConfiguration => self.handle_write_active_configuration(),
            Command::Reset => self.handle_reset(),
            Command::GetIdentificationInformation => self.handle_get_identification_information(),
            Command::SetAutoAnswer(_) => self.handle_set_auto_answer(),
            Command::SetCommandTerminationCharacter(_) => {
                self.handle_set_command_termination_character()
            }
            Command::SetResponseFormattingCharacter(_) => {
                self.handle_set_response_formatting_character()
            }
            Command::SetCommandLineEditingCharacter(_) => {
                self.handle_set_command_line_editing_character()
            }
            Command::SetPauseBeforeBlindDialing(_) => self.handle_set_pause_before_blind_dialing(),
            Command::SetConnectionCompletionTimeout(_) => {
                self.handle_set_connection_completion_timeout()
            }
            Command::SetCommaDialModifierTime(_) => self.handle_set_comma_dial_modifier_time(),
            Command::SetAutomaticDisconnectDelay(_) => self.handle_set_automatic_disconnect_delay(),
            _ => ExecutionResult::Unhandled,
        }
    }
}
