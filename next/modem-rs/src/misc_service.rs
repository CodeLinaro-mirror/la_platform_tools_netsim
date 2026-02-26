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
    speaker_volume: Mutex<u8>,
    speaker_mute: Mutex<u8>,
    quiet_mode: Mutex<u8>,
    verbose_mode: Mutex<u8>,
    icf_format: Mutex<u8>,
    icf_parity: Mutex<u8>,
    ifc_dce: Mutex<u8>,
    ifc_dte: Mutex<u8>,
}

impl MiscService {
    pub fn new() -> Self {
        Self {
            clock: Mutex::new("".to_string()),
            speaker_volume: Mutex::new(1),
            speaker_mute: Mutex::new(1),
            quiet_mode: Mutex::new(0),
            verbose_mode: Mutex::new(1),
            icf_format: Mutex::new(3),
            icf_parity: Mutex::new(3),
            ifc_dce: Mutex::new(2),
            ifc_dte: Mutex::new(2),
        }
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

    pub fn handle_set_icf(&self, format: u8, parity: u8) -> ExecutionResult {
        *self.icf_format.lock().unwrap() = format;
        *self.icf_parity.lock().unwrap() = parity;
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_ifc(&self, dce: u8, dte: u8) -> ExecutionResult {
        *self.ifc_dce.lock().unwrap() = dce;
        *self.ifc_dte.lock().unwrap() = dte;
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

    pub fn handle_set_speaker_volume(&self, vol: u8) -> ExecutionResult {
        *self.speaker_volume.lock().unwrap() = vol;
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_speaker_mute(&self, mute: u8) -> ExecutionResult {
        *self.speaker_mute.lock().unwrap() = mute;
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_quiet_mode(&self, quiet: u8) -> ExecutionResult {
        *self.quiet_mode.lock().unwrap() = quiet;
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_set_verbose_mode(&self, verbose: u8) -> ExecutionResult {
        *self.verbose_mode.lock().unwrap() = verbose;
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_reset_to_factory_defaults(&self) -> ExecutionResult {
        *self.speaker_volume.lock().unwrap() = 1;
        *self.speaker_mute.lock().unwrap() = 1;
        *self.quiet_mode.lock().unwrap() = 0;
        *self.verbose_mode.lock().unwrap() = 1;
        *self.icf_format.lock().unwrap() = 3;
        *self.icf_parity.lock().unwrap() = 3;
        *self.ifc_dce.lock().unwrap() = 2;
        *self.ifc_dte.lock().unwrap() = 2;
        ExecutionResult::Handled(HandledCommand::ok())
    }

    pub fn handle_view_active_configuration(&self) -> ExecutionResult {
        let vol = *self.speaker_volume.lock().unwrap();
        let mute = *self.speaker_mute.lock().unwrap();
        let quiet = *self.quiet_mode.lock().unwrap();
        let verbose = *self.verbose_mode.lock().unwrap();
        let icf_fmt = *self.icf_format.lock().unwrap();
        let icf_par = *self.icf_parity.lock().unwrap();
        let ifc_dce = *self.ifc_dce.lock().unwrap();
        let ifc_dte = *self.ifc_dte.lock().unwrap();

        let config_str = format!(
            "ACTIVE PROFILE:\nL:{} M:{} Q:{} V:{} ICF:{},{} IFC:{},{}\nOK\r\n",
            vol, mute, quiet, verbose, icf_fmt, icf_par, ifc_dce, ifc_dte
        );

        ExecutionResult::Handled(HandledCommand { responses: vec![config_str], action: None })
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
            Command::SetTeTaControlCharacterFraming(f, p) => self.handle_set_icf(*f, *p),
            Command::SetTeTaLocalDataFlowControl(d1, d2) => self.handle_set_ifc(*d1, *d2),
            Command::SetTeTaFixedLocalRate(_) => self.handle_set_ipr(),
            Command::SetTime(time) => self.handle_set_time(*time),
            Command::QueryTime => self.handle_query_time(),
            Command::SetReportMobileEquipmentError(_) => {
                self.handle_set_report_mobile_equipment_error()
            }
            Command::SetEcho(_) => self.handle_set_echo(),
            Command::SetSpeakerVolume(vol) => self.handle_set_speaker_volume(*vol),
            Command::SetSpeakerMute(mute) => self.handle_set_speaker_mute(*mute),
            Command::SetQuietMode(quiet) => self.handle_set_quiet_mode(*quiet),
            Command::SetVerboseMode(verbose) => self.handle_set_verbose_mode(*verbose),
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
