// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use modem_rs_derive::CommandParser;

use crate::{
    parser::QuotedString,
    types::{
        CallMode, CmeeMode, ExecutionResult, FlowControlMode, IcfFormat, IcfParity, Parsable,
        ProductSerialNumberType, SpeakerMuteMode,
    },
};

const DEFAULT_IMEI: &str = "867400022047199";
const DEFAULT_SVN: &str = "01";
const DEFAULT_INFO: &str = "modem simulator";

/// Miscellaneous modem service AT commands.
#[derive(Debug, PartialEq, Clone, Copy, CommandParser)]
pub enum MiscCommand<'a> {
    #[command(tag = "AT+CMEE=?")]
    QuerySupportedReportMobileEquipmentError,
    #[command(tag = "AT+CMEE?")]
    QueryReportMobileEquipmentError,
    #[command(tag = "AT+CMEE=")]
    SetReportMobileEquipmentError(CmeeMode),
    /// Goldfish specific concatenated init command
    #[command(tag = "ATE0Q0V1")]
    GoldfishInitSequence,
    #[command(tag = "ATE")]
    SetEcho(bool),
    #[command(tag = "ATL")]
    SetSpeakerVolume(u8),
    #[command(tag = "ATM")]
    SetSpeakerMute(SpeakerMuteMode),
    #[command(tag = "ATQ")]
    SetQuietMode(bool),
    #[command(tag = "ATV")]
    SetVerboseMode(bool),
    #[command(tag = "AT&F")]
    ResetToFactoryDefaults,
    #[command(tag = "AT&V")]
    ViewActiveConfiguration,
    #[command(tag = "AT&W")]
    WriteActiveConfiguration,
    #[command(tag = "ATZ")]
    Reset,
    #[command(tag = "ATI")]
    GetIdentificationInformation,
    #[command(tag = "ATS0=")]
    SetAutoAnswer(u8),
    #[command(tag = "ATS3=")]
    SetCommandTerminationCharacter(u8),
    #[command(tag = "ATS4=")]
    SetResponseFormattingCharacter(u8),
    #[command(tag = "ATS5=")]
    SetCommandLineEditingCharacter(u8),
    #[command(tag = "ATS6=")]
    SetPauseBeforeBlindDialing(u8),
    #[command(tag = "ATS7=")]
    SetConnectionCompletionTimeout(u8),
    #[command(tag = "ATS8=")]
    SetCommaDialModifierTime(u8),
    #[command(tag = "ATS10=")]
    SetAutomaticDisconnectDelay(u8),
    #[command(tag = "AT+GCAP")]
    GetCapabilities,
    #[command(tag = "AT+GMI")]
    GetManufacturerIdentification,
    #[command(tag = "AT+GMM")]
    GetModelId,
    #[command(tag = "AT+GMR")]
    GetRevision,
    #[command(tag = "AT+GSN")]
    GetSerialNumber,
    #[command(tag = "AT+CGSN=")]
    GetProductSerialNumberGsmWithType(ProductSerialNumberType),
    #[command(tag = "AT+CGSN")]
    GetProductSerialNumberGsm,
    #[command(tag = "AT+ICF=")]
    SetTeTaControlCharacterFraming(IcfFormat, Option<IcfParity>),
    #[command(tag = "AT+IFC=")]
    SetTeTaLocalDataFlowControl(FlowControlMode, Option<FlowControlMode>),
    #[command(tag = "AT+IPR=")]
    SetTeTaFixedLocalRate(u32),
    #[command(tag = "AT+CCLK=")]
    SetTime(QuotedString<'a>),
    #[command(tag = "AT+CCLK?")]
    QueryTime,
    #[command(tag = "AT+CMOD=")]
    SetCallMode(CallMode),
    #[command(tag = "AT+CSCS=")]
    SetCharacterSet(QuotedString<'a>),
    #[command(tag = "AT")]
    Test,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MiscResponse {
    Clock(String),
    ModelId(String),
    Revision(String),
    SerialNumber(String),
    ProductSerialNumberGsm(ProductSerialNumberType),
    ErrorReportingMode(CmeeMode),
    ErrorReportingSupported,
    ActiveConfiguration {
        speaker_volume: u8,
        speaker_mute: SpeakerMuteMode,
        quiet_mode: bool,
        verbose_mode: bool,
        icf_format: IcfFormat,
        icf_parity: IcfParity,
        ifc_dce: FlowControlMode,
        ifc_dte: FlowControlMode,
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
                ProductSerialNumberType::ImeiWithInfo => {
                    write!(f, "{DEFAULT_IMEI}{DEFAULT_INFO}\r\n")
                }
                ProductSerialNumberType::Imei => write!(f, "{DEFAULT_IMEI}\r\n"),
                ProductSerialNumberType::ImeiWithSvn => {
                    write!(f, "{DEFAULT_IMEI}{DEFAULT_SVN}\r\n")
                }
                ProductSerialNumberType::Svn => write!(f, "{DEFAULT_SVN}\r\n"),
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
                let quiet = if *quiet_mode { 1 } else { 0 };
                let verbose = if *verbose_mode { 1 } else { 0 };
                write!(
                    f,
                    "ACTIVE PROFILE:\r\nL:{speaker_volume} M:{speaker_mute} Q:{quiet} V:{verbose} ICF:{icf_format},{icf_parity} IFC:{ifc_dce},{ifc_dte}\r\n"
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
    speaker_mute: SpeakerMuteMode,
    quiet_mode: bool,
    verbose_mode: bool,
    icf_format: IcfFormat,
    icf_parity: IcfParity,
    ifc_dce: FlowControlMode,
    ifc_dte: FlowControlMode,
}

impl Default for MiscService {
    fn default() -> Self {
        Self {
            cmee_mode: CmeeMode::default(),
            clock: "".to_string(),
            speaker_volume: 1,
            speaker_mute: SpeakerMuteMode::OnAndOffOnCarrier,
            quiet_mode: false,
            verbose_mode: true,
            icf_format: IcfFormat::Data8Stop1,
            icf_parity: IcfParity::Space,
            ifc_dce: FlowControlMode::Hardware,
            ifc_dte: FlowControlMode::Hardware,
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
        Ok(Some(MiscResponse::ProductSerialNumberGsm(ProductSerialNumberType::Imei)))
    }

    fn handle_get_product_serial_number_gsm_with_type(
        &self,
        snt: ProductSerialNumberType,
    ) -> MiscResult {
        Ok(Some(MiscResponse::ProductSerialNumberGsm(snt)))
    }

    fn handle_set_icf(&mut self, format: IcfFormat, parity: Option<IcfParity>) -> MiscResult {
        self.icf_format = format;
        if let Some(p) = parity {
            self.icf_parity = p;
        }
        Ok(None)
    }

    fn handle_set_ifc(&mut self, dce: FlowControlMode, dte: Option<FlowControlMode>) -> MiscResult {
        self.ifc_dce = dce;
        if let Some(d) = dte {
            self.ifc_dte = d;
        }
        Ok(None)
    }

    fn handle_set_report_mobile_equipment_error(&mut self, mode: CmeeMode) -> MiscResult {
        self.cmee_mode = mode;
        Ok(None)
    }

    fn handle_query_report_mobile_equipment_error(&self) -> MiscResult {
        Ok(Some(MiscResponse::ErrorReportingMode(self.cmee_mode)))
    }

    fn handle_query_supported_report_mobile_equipment_error(&self) -> MiscResult {
        Ok(Some(MiscResponse::ErrorReportingSupported))
    }

    fn handle_goldfish_init_sequence(&mut self) -> MiscResult {
        self.quiet_mode = false;
        self.verbose_mode = true;
        Ok(None)
    }

    fn handle_reset_to_factory_defaults(&mut self) -> MiscResult {
        self.cmee_mode = CmeeMode::default();
        self.speaker_volume = 1;
        self.speaker_mute = SpeakerMuteMode::OnAndOffOnCarrier;
        self.quiet_mode = false;
        self.verbose_mode = true;
        self.icf_format = IcfFormat::Data8Stop1;
        self.icf_parity = IcfParity::Space;
        self.ifc_dce = FlowControlMode::Hardware;
        self.ifc_dte = FlowControlMode::Hardware;
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

    fn handle_get_manufacturer_identification(&self) -> MiscResult {
        Ok(Some(MiscResponse::ManufacturerIdentification("Android".to_string())))
    }

    fn handle_get_capabilities(&self) -> MiscResult {
        Ok(Some(MiscResponse::Capabilities))
    }

    pub fn execute<'a>(&mut self, command: &MiscCommand<'a>) -> ExecutionResult {
        let misc_result = match command {
            MiscCommand::GetManufacturerIdentification => {
                self.handle_get_manufacturer_identification()
            }
            MiscCommand::GetCapabilities => self.handle_get_capabilities(),
            MiscCommand::GetModelId => self.handle_get_model_id(),
            MiscCommand::GetRevision => self.handle_get_revision(),
            MiscCommand::GetSerialNumber => self.handle_get_serial_number(),
            MiscCommand::GetProductSerialNumberGsm => self.handle_get_product_serial_number_gsm(),
            MiscCommand::GetProductSerialNumberGsmWithType(snt) => {
                self.handle_get_product_serial_number_gsm_with_type(*snt)
            }
            MiscCommand::SetTeTaControlCharacterFraming(f, p) => self.handle_set_icf(*f, *p),
            MiscCommand::SetTeTaLocalDataFlowControl(d1, d2) => self.handle_set_ifc(*d1, *d2),
            MiscCommand::SetTeTaFixedLocalRate(_) => Ok(None),
            MiscCommand::SetTime(time) => self.handle_set_time(*time),
            MiscCommand::QueryTime => self.handle_query_time(),
            MiscCommand::SetReportMobileEquipmentError(mode) => {
                self.handle_set_report_mobile_equipment_error(*mode)
            }
            MiscCommand::QueryReportMobileEquipmentError => {
                self.handle_query_report_mobile_equipment_error()
            }
            MiscCommand::QuerySupportedReportMobileEquipmentError => {
                self.handle_query_supported_report_mobile_equipment_error()
            }
            MiscCommand::GoldfishInitSequence => self.handle_goldfish_init_sequence(),
            MiscCommand::SetSpeakerVolume(vol) => {
                self.speaker_volume = *vol;
                Ok(None)
            }
            MiscCommand::SetSpeakerMute(mute) => {
                self.speaker_mute = *mute;
                Ok(None)
            }
            MiscCommand::SetQuietMode(quiet) => {
                self.quiet_mode = *quiet;
                Ok(None)
            }
            MiscCommand::SetVerboseMode(verbose) => {
                self.verbose_mode = *verbose;
                Ok(None)
            }
            MiscCommand::ResetToFactoryDefaults => self.handle_reset_to_factory_defaults(),
            MiscCommand::ViewActiveConfiguration => self.handle_view_active_configuration(),
            MiscCommand::WriteActiveConfiguration
            | MiscCommand::Reset
            | MiscCommand::GetIdentificationInformation
            | MiscCommand::SetAutoAnswer(_)
            | MiscCommand::SetCommandTerminationCharacter(_)
            | MiscCommand::SetResponseFormattingCharacter(_)
            | MiscCommand::SetCommandLineEditingCharacter(_)
            | MiscCommand::SetPauseBeforeBlindDialing(_)
            | MiscCommand::SetConnectionCompletionTimeout(_)
            | MiscCommand::SetCommaDialModifierTime(_)
            | MiscCommand::SetAutomaticDisconnectDelay(_)
            | MiscCommand::SetCallMode(_)
            | MiscCommand::SetCharacterSet(_)
            | MiscCommand::SetEcho(_)
            | MiscCommand::Test => Ok(None),
        };

        misc_result.into()
    }
}
