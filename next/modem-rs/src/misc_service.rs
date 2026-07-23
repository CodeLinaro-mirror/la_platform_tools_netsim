// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use crate::{
    parser::{Command, QuotedString},
    types::{CmeeMode, ExecutionResult},
};

const DEFAULT_IMEI: &str = "867400022047199";
const DEFAULT_SVN: &str = "01";
const DEFAULT_INFO: &str = "modem simulator";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MiscResponse {
    Clock(String),
    ModelId(String),
    Revision(String),
    SerialNumber(String),
    ProductSerialNumberGsm(u8),
    ErrorReportingMode(u8),
    ErrorReportingSupported,
    ActiveConfiguration {
        speaker_volume: u8,
        speaker_mute: u8,
        quiet_mode: u8,
        verbose_mode: u8,
        icf_format: u8,
        icf_parity: u8,
        ifc_dce: u8,
        ifc_dte: u8,
    },
    ManufacturerIdentification(String),
    Capabilities,
}

impl std::fmt::Display for MiscResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MiscResponse::Clock(clock) => write!(f, "+CCLK: \"{clock}\"\r\n"),
            MiscResponse::ModelId(model_id) => write!(f, "{model_id}\r\n"),
            MiscResponse::Revision(revision) => write!(f, "{revision}\r\n"),
            MiscResponse::SerialNumber(serial_number) => write!(f, "{serial_number}\r\n"),
            MiscResponse::ProductSerialNumberGsm(snt) => match snt {
                0 => write!(f, "{DEFAULT_IMEI}{DEFAULT_INFO}\r\n"),
                1 => write!(f, "{DEFAULT_IMEI}\r\n"),
                2 => write!(f, "{DEFAULT_IMEI}{DEFAULT_SVN}\r\n"),
                3 => write!(f, "{DEFAULT_SVN}\r\n"),
                _ => write!(f, "{DEFAULT_IMEI}\r\n"),
            },
            MiscResponse::ErrorReportingMode(mode) => write!(f, "+CMEE: {mode}\r\n"),
            MiscResponse::ErrorReportingSupported => write!(f, "+CMEE: (0-2)\r\n"),
            MiscResponse::ActiveConfiguration {
                speaker_volume,
                speaker_mute,
                quiet_mode,
                verbose_mode,
                icf_format,
                icf_parity,
                ifc_dce,
                ifc_dte,
            } => {
                write!(
                    f,
                    "ACTIVE PROFILE:\r\nL:{speaker_volume} M:{speaker_mute} Q:{quiet_mode} V:{verbose_mode} ICF:{icf_format},{icf_parity} IFC:{ifc_dce},{ifc_dte}\r\n"
                )
            }
            MiscResponse::ManufacturerIdentification(manufacturer) => {
                write!(f, "{manufacturer}\r\n")
            }
            MiscResponse::Capabilities => write!(f, "+GCAP: +FCLASS,+DS\r\n"),
        }
    }
}

type MiscResult = Result<Option<MiscResponse>, ExecutionResult>;

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

    fn handle_set_time(&mut self, time: QuotedString) -> MiscResult {
        self.clock = String::from_utf8(time.to_vec()).unwrap_or_default();
        Ok(None)
    }

    pub fn set_time(&mut self, time: String) {
        self.clock = time;
    }

    fn handle_query_time(&self) -> MiscResult {
        Ok(Some(MiscResponse::Clock(self.clock.clone())))
    }

    fn handle_get_model_id(&self) -> MiscResult {
        Ok(Some(MiscResponse::ModelId("gLinux".to_string())))
    }

    fn handle_get_revision(&self) -> MiscResult {
        Ok(Some(MiscResponse::Revision("1.0".to_string())))
    }

    fn handle_get_serial_number(&self) -> MiscResult {
        Ok(Some(MiscResponse::SerialNumber("0123456789".to_string())))
    }

    fn handle_get_product_serial_number_gsm(&self) -> MiscResult {
        Ok(Some(MiscResponse::ProductSerialNumberGsm(1)))
    }

    fn handle_get_product_serial_number_gsm_with_type(&self, snt: u8) -> MiscResult {
        Ok(Some(MiscResponse::ProductSerialNumberGsm(snt)))
    }

    fn handle_set_icf(&mut self, format: u8, parity: u8) -> MiscResult {
        self.icf_format = format;
        self.icf_parity = parity;
        Ok(None)
    }

    fn handle_set_ifc(&mut self, dce: u8, dte: u8) -> MiscResult {
        self.ifc_dce = dce;
        self.ifc_dte = dte;
        Ok(None)
    }

    fn handle_set_ipr(&self) -> MiscResult {
        Ok(None)
    }

    fn handle_set_report_mobile_equipment_error(&mut self, mode: u8) -> MiscResult {
        if let Some(m) = CmeeMode::from_u8(mode) {
            self.cmee_mode = m;
            Ok(None)
        } else {
            Err(ExecutionResult::error())
        }
    }

    fn handle_query_report_mobile_equipment_error(&self) -> MiscResult {
        Ok(Some(MiscResponse::ErrorReportingMode(self.cmee_mode as u8)))
    }

    fn handle_query_supported_report_mobile_equipment_error(&self) -> MiscResult {
        Ok(Some(MiscResponse::ErrorReportingSupported))
    }

    fn handle_goldfish_init_sequence(&self) -> MiscResult {
        Ok(None)
    }

    fn handle_set_echo(&self) -> MiscResult {
        Ok(None)
    }

    fn handle_set_speaker_volume(&mut self, vol: u8) -> MiscResult {
        self.speaker_volume = vol;
        Ok(None)
    }

    fn handle_set_speaker_mute(&mut self, mute: u8) -> MiscResult {
        self.speaker_mute = mute;
        Ok(None)
    }

    fn handle_set_quiet_mode(&mut self, quiet: u8) -> MiscResult {
        self.quiet_mode = quiet;
        Ok(None)
    }

    fn handle_set_verbose_mode(&mut self, verbose: u8) -> MiscResult {
        self.verbose_mode = verbose;
        Ok(None)
    }

    fn handle_reset_to_factory_defaults(&mut self) -> MiscResult {
        self.cmee_mode = CmeeMode::default();
        self.speaker_volume = 1;
        self.speaker_mute = 1;
        self.quiet_mode = 0;
        self.verbose_mode = 1;
        self.icf_format = 3;
        self.icf_parity = 3;
        self.ifc_dce = 2;
        self.ifc_dte = 2;
        Ok(None)
    }

    fn handle_view_active_configuration(&self) -> MiscResult {
        Ok(Some(MiscResponse::ActiveConfiguration {
            speaker_volume: self.speaker_volume,
            speaker_mute: self.speaker_mute,
            quiet_mode: self.quiet_mode,
            verbose_mode: self.verbose_mode,
            icf_format: self.icf_format,
            icf_parity: self.icf_parity,
            ifc_dce: self.ifc_dce,
            ifc_dte: self.ifc_dte,
        }))
    }

    fn handle_write_active_configuration(&self) -> MiscResult {
        Ok(None)
    }

    fn handle_reset(&self) -> MiscResult {
        Ok(None)
    }

    fn handle_get_identification_information(&self) -> MiscResult {
        Ok(None)
    }

    fn handle_set_auto_answer(&self) -> MiscResult {
        Ok(None)
    }

    fn handle_set_command_termination_character(&self) -> MiscResult {
        Ok(None)
    }

    fn handle_set_response_formatting_character(&self) -> MiscResult {
        Ok(None)
    }

    fn handle_set_command_line_editing_character(&self) -> MiscResult {
        Ok(None)
    }

    fn handle_set_pause_before_blind_dialing(&self) -> MiscResult {
        Ok(None)
    }

    fn handle_set_connection_completion_timeout(&self) -> MiscResult {
        Ok(None)
    }

    fn handle_set_comma_dial_modifier_time(&self) -> MiscResult {
        Ok(None)
    }

    fn handle_set_automatic_disconnect_delay(&self) -> MiscResult {
        Ok(None)
    }

    fn handle_set_call_mode(&self) -> MiscResult {
        Ok(None)
    }

    fn handle_set_character_set(&self) -> MiscResult {
        Ok(None)
    }

    fn handle_get_manufacturer_identification(&self) -> MiscResult {
        Ok(Some(MiscResponse::ManufacturerIdentification("Android".to_string())))
    }

    fn handle_get_capabilities(&self) -> MiscResult {
        Ok(Some(MiscResponse::Capabilities))
    }

    pub fn execute(&mut self, command: &Command) -> ExecutionResult {
        let misc_result = match command {
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
            Command::Test => Ok(None),
            _ => Err(ExecutionResult::Unhandled),
        };

        misc_result.into()
    }
}
