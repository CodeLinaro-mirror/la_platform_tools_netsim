// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{
    parser::{Command, QuotedString},
    types::{CmeeMode, ExecutionResult, HandledCommand},
};

const DEFAULT_IMEI: &str = "867400022047199";
const DEFAULT_IMEI_CRLF: &str = "867400022047199\r\n";
const DEFAULT_SVN: &str = "01";
const DEFAULT_SVN_CRLF: &str = "01\r\n";
const DEFAULT_INFO: &str = "modem simulator";

pub struct MiscService {
    cmee_mode: CmeeMode,
    clock: String,
    speaker_volume: u8,
    speaker_mute: u8,
    quiet_mode: u8,
    verbose_mode: u8,
    icf_format: u8,
    icf_parity: u8,
    ifc_dce: u8,
    ifc_dte: u8,
}

impl Default for MiscService {
    fn default() -> Self {
        Self {
            cmee_mode: CmeeMode::default(),
            clock: "".to_string(),
            speaker_volume: 1,
            speaker_mute: 1,
            quiet_mode: 0,
            verbose_mode: 1,
            icf_format: 3,
            icf_parity: 3,
            ifc_dce: 2,
            ifc_dte: 2,
        }
    }
}

impl MiscService {
    pub fn cmee_mode(&self) -> CmeeMode {
        self.cmee_mode
    }

    // --- Pure command handlers ---

    pub fn handle_set_time(&mut self, time: QuotedString) -> ExecutionResult {
        self.clock = String::from_utf8(time.to_vec()).unwrap_or_default();
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn set_time(&mut self, time: String) {
        self.clock = time;
    }

    pub fn handle_query_time(&self) -> ExecutionResult {
        let response_data = format!("+CCLK: \"{}\"\r\n", self.clock);
        ExecutionResult::Success(HandledCommand {
            responses: vec![response_data, "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_get_model_id(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand {
            responses: vec!["gLinux\r\n".to_string(), "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_get_revision(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand {
            responses: vec!["1.0\r\n".to_string(), "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_get_serial_number(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand {
            responses: vec!["0123456789\r\n".to_string(), "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_get_product_serial_number_gsm(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand {
            responses: vec![DEFAULT_IMEI_CRLF.to_string(), "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_get_product_serial_number_gsm_with_type(&self, snt: u8) -> ExecutionResult {
        let response = match snt {
            0 => format!("{DEFAULT_IMEI}{DEFAULT_INFO}\r\n"),
            1 => DEFAULT_IMEI_CRLF.to_string(),
            2 => format!("{DEFAULT_IMEI}{DEFAULT_SVN}\r\n"),
            3 => DEFAULT_SVN_CRLF.to_string(),
            _ => DEFAULT_IMEI_CRLF.to_string(),
        };

        ExecutionResult::Success(HandledCommand {
            responses: vec![response, "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_set_icf(&mut self, format: u8, parity: u8) -> ExecutionResult {
        self.icf_format = format;
        self.icf_parity = parity;
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_set_ifc(&mut self, dce: u8, dte: u8) -> ExecutionResult {
        self.ifc_dce = dce;
        self.ifc_dte = dte;
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_set_ipr(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_set_report_mobile_equipment_error(&mut self, mode: u8) -> ExecutionResult {
        if let Some(m) = CmeeMode::from_u8(mode) {
            self.cmee_mode = m;
            ExecutionResult::Success(HandledCommand::ok())
        } else {
            ExecutionResult::Error
        }
    }

    pub fn handle_query_report_mobile_equipment_error(&self) -> ExecutionResult {
        let response_data = format!("+CMEE: {}\r\n", self.cmee_mode as u8);
        ExecutionResult::Success(HandledCommand {
            responses: vec![response_data, "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_query_supported_report_mobile_equipment_error(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand {
            responses: vec!["+CMEE: (0-2)\r\n".to_string(), "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_goldfish_init_sequence(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_set_echo(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_set_speaker_volume(&mut self, vol: u8) -> ExecutionResult {
        self.speaker_volume = vol;
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_set_speaker_mute(&mut self, mute: u8) -> ExecutionResult {
        self.speaker_mute = mute;
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_set_quiet_mode(&mut self, quiet: u8) -> ExecutionResult {
        self.quiet_mode = quiet;
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_set_verbose_mode(&mut self, verbose: u8) -> ExecutionResult {
        self.verbose_mode = verbose;
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_reset_to_factory_defaults(&mut self) -> ExecutionResult {
        self.cmee_mode = CmeeMode::default();
        self.speaker_volume = 1;
        self.speaker_mute = 1;
        self.quiet_mode = 0;
        self.verbose_mode = 1;
        self.icf_format = 3;
        self.icf_parity = 3;
        self.ifc_dce = 2;
        self.ifc_dte = 2;
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_view_active_configuration(&self) -> ExecutionResult {
        let config_str = format!(
            "ACTIVE PROFILE:\r\nL:{} M:{} Q:{} V:{} ICF:{},{} IFC:{},{}\r\nOK\r\n",
            self.speaker_volume,
            self.speaker_mute,
            self.quiet_mode,
            self.verbose_mode,
            self.icf_format,
            self.icf_parity,
            self.ifc_dce,
            self.ifc_dte
        );

        ExecutionResult::Success(HandledCommand { responses: vec![config_str], action: None })
    }

    pub fn handle_write_active_configuration(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_reset(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_get_identification_information(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_set_auto_answer(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_set_command_termination_character(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_set_response_formatting_character(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_set_command_line_editing_character(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_set_pause_before_blind_dialing(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_set_connection_completion_timeout(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_set_comma_dial_modifier_time(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_set_automatic_disconnect_delay(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_set_call_mode(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_set_character_set(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand::ok())
    }

    pub fn handle_get_manufacturer_identification(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand {
            responses: vec!["Android\r\n".to_string(), "OK\r\n".to_string()],
            action: None,
        })
    }

    pub fn handle_get_capabilities(&self) -> ExecutionResult {
        ExecutionResult::Success(HandledCommand {
            responses: vec!["+GCAP: +FCLASS,+DS\r\nOK\r\n".to_string()],
            action: None,
        })
    }

    pub fn execute(&mut self, command: &Command) -> ExecutionResult {
        match command {
            Command::GetManufacturerIdentification => self.handle_get_manufacturer_identification(),
            Command::GetCapabilities => self.handle_get_capabilities(),
            Command::GetModelId => self.handle_get_model_id(),
            Command::GetRevision => self.handle_get_revision(),
            Command::GetSerialNumber => self.handle_get_serial_number(),
            Command::GetProductSerialNumberGsm => self.handle_get_product_serial_number_gsm(),
            Command::GetProductSerialNumberGsmWithType(snt) => {
                self.handle_get_product_serial_number_gsm_with_type(*snt)
            }
            Command::SetTeTaControlCharacterFraming(f, p) => self.handle_set_icf(*f, *p),
            Command::SetTeTaLocalDataFlowControl(d1, d2) => self.handle_set_ifc(*d1, *d2),
            Command::SetTeTaFixedLocalRate(_) => self.handle_set_ipr(),
            Command::SetTime(time) => self.handle_set_time(*time),
            Command::QueryTime => self.handle_query_time(),
            Command::SetReportMobileEquipmentError(mode) => {
                self.handle_set_report_mobile_equipment_error(*mode)
            }
            Command::QueryReportMobileEquipmentError => {
                self.handle_query_report_mobile_equipment_error()
            }
            Command::QuerySupportedReportMobileEquipmentError => {
                self.handle_query_supported_report_mobile_equipment_error()
            }
            Command::GoldfishInitSequence => self.handle_goldfish_init_sequence(),
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
            Command::SetCallMode(_) => self.handle_set_call_mode(),
            Command::SetCharacterSet(_) => self.handle_set_character_set(),
            Command::Test => ExecutionResult::Success(HandledCommand::ok()),
            _ => ExecutionResult::Unhandled,
        }
    }
}
