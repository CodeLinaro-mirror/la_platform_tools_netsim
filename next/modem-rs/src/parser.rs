// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use modem_rs_derive::CommandParser;
use nom::{IResult, bytes::complete::tag};

use crate::types::Parsable;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct QuotedString<'a>(pub &'a [u8]);

impl<'a> QuotedString<'a> {
    pub fn to_vec(self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl<'a> AsRef<[u8]> for QuotedString<'a> {
    fn as_ref(&self) -> &[u8] {
        self.0
    }
}

impl<'a> std::ops::Deref for QuotedString<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a> Parsable<'a> for QuotedString<'a> {
    fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self> {
        use nom::{bytes::complete::take_while, sequence::delimited};
        let (input, content) =
            delimited(tag(br#"""#), take_while(|c| c != b'"'), tag(br#"""#))(input)?;
        Ok((input, QuotedString(content)))
    }
}

pub fn parse_raw_data(input: &[u8]) -> IResult<&[u8], &[u8]> {
    use nom::bytes::complete::take_while;
    let (input, content) = take_while(|c: u8| c != b'\r' && c != b'\n')(input)?;
    Ok((input, content))
}

pub fn parse_until_semicolon(input: &[u8]) -> IResult<&[u8], &[u8]> {
    use nom::{
        bytes::complete::{tag, take_while},
        combinator::opt,
    };
    let (input, content) = take_while(|c: u8| c != b';' && c != b'\r' && c != b'\n')(input)?;
    let (input, _) = opt(tag(b";"))(input)?;
    Ok((input, content))
}

/// 3GPP TS 27.007 commands, with some vendor-specific commands.
#[derive(Debug, PartialEq, Clone, Copy, CommandParser)]
pub enum Command<'a> {
    /// Get PIN status
    #[command(tag = "AT+CPIN?")]
    GetSimStatus,
    /// Enter PIN or PUK
    #[command(tag = "AT+CPIN=")]
    EnterPin(QuotedString<'a>, Option<QuotedString<'a>>),
    /// Restricted SIM access
    #[command(tag = "AT+CRSM=")]
    SimIo { command: u16, file_id: u16, p1: u8, p2: u8, p3: u8, data: Option<QuotedString<'a>> },
    /// Request International Mobile Subscriber Identity
    #[command(tag = "AT+CIMI")]
    GetImsi,
    /// Request ICCID
    #[command(tag = "AT+CICCID")]
    GetIccid,
    /// Open logical channel
    #[command(tag = "AT+CCHO=")]
    OpenLogicalChannel(#[parser(parse_raw_data)] &'a [u8]),
    /// Close logical channel
    #[command(tag = "AT+CCHC=")]
    CloseLogicalChannel(u8),
    /// Transmit APDU on logical channel
    #[command(tag = "AT+CGLA=")]
    TransmitLogicalChannel(u8, u8, #[parser(parse_raw_data)] &'a [u8]),
    /// Change password
    #[command(tag = "AT+CPWD=")]
    ChangePassword(QuotedString<'a>, QuotedString<'a>, QuotedString<'a>),
    /// VENDOR: Query PIN retries
    #[command(tag = "AT+SPIC")]
    QueryPinRetries,
    /// 3GPP2 C.S0023: Set CDMA subscription source
    #[command(tag = "AT+CCSS=")]
    SetCdmaSubscriptionSource(u8),
    /// 3GPP2 C.S0023: Query CDMA subscription source
    #[command(tag = "AT+CCSS?")]
    QueryCdmaSubscriptionSource,
    /// 3GPP2 C.S0023: Set CDMA roaming preference
    #[command(tag = "AT+WRMP=")]
    SetCdmaRoamingPreference(u8),
    /// 3GPP2 C.S0023: Query CDMA roaming preference
    #[command(tag = "AT+WRMP?")]
    QueryCdmaRoamingPreference,
    /// SIM authentication
    #[command(tag = "AT+MBAU=")]
    SimAuthentication(#[parser(parse_raw_data)] &'a [u8]),
    /// VENDOR: Update phone number
    #[command(tag = "AT+REMOTEUPADATEPHONENUMBER")]
    UpdatePhoneNumber(#[parser(parse_raw_data)] &'a [u8]),
    /// Dial
    #[command(tag = "ATD")]
    Dial(#[parser(parse_until_semicolon)] &'a [u8]),
    /// Answer
    #[command(tag = "ATA")]
    Answer,
    /// Hangup
    #[command(tag = "ATH")]
    Hangup,
    /// Call hold and multiparty
    #[command(tag = "AT+CHLD=")]
    CallHold(u8),
    /// List current calls
    #[command(tag = "AT+CLCC")]
    QueryCurrentCalls,
    /// Mute control
    #[command(tag = "AT+CMUT=")]
    SetMute(u8),
    /// Query mute status
    #[command(tag = "AT+CMUT?")]
    QueryMute,
    /// Send DTMF
    #[command(tag = "AT+VTS=")]
    SendDtmf(#[parser(parse_raw_data)] &'a [u8]),
    /// VENDOR: Set emergency mode
    #[command(tag = "AT+WSOS=")]
    SetEmergencyMode(u8),
    /// VENDOR: Query emergency mode
    #[command(tag = "AT+WSOS?")]
    QueryEmergencyMode,
    /// VENDOR: Remote call
    #[command(tag = "AT+REMOTECALL=")]
    RemoteCall(#[parser(parse_raw_data)] &'a [u8]),
    /// VENDOR: Ring indication
    #[command(tag = "RING")]
    Ring,
    /// Operator selection query
    #[command(tag = "AT+COPS?")]
    QueryOperator,
    /// Set operator selection
    #[command(tag = "AT+COPS=")]
    SetOperator { mode: u8, format: Option<u8>, oper: Option<QuotedString<'a>> },
    /// Query voice network registration
    #[command(tag = "AT+CREG?")]
    QueryVoiceNetworkRegistration,
    /// Set voice network registration
    #[command(tag = "AT+CREG=")]
    SetVoiceNetworkRegistration(u8),
    /// Query data network registration
    #[command(tag = "AT+CGREG?")]
    QueryDataNetworkRegistration,
    /// Set data network registration
    #[command(tag = "AT+CGREG=")]
    SetDataNetworkRegistration(u8),
    /// Query LTE network registration
    #[command(tag = "AT+CEREG?")]
    QueryLteNetworkRegistration,
    /// Set LTE network registration
    #[command(tag = "AT+CEREG=")]
    SetLteNetworkRegistration(u8),
    /// Query radio power state
    #[command(tag = "AT+CFUN?")]
    QueryRadioPower,
    /// Set radio power state
    #[command(tag = "AT+CFUN=")]
    SetRadioPower(u8),
    /// Signal quality
    #[command(tag = "AT+CSQ")]
    QuerySignalStrength,
    /// Extended signal quality
    #[command(tag = "AT+CESQ")]
    QueryExtendedSignalQuality,
    /// 3GPP TS 27.005: Send message
    #[command(tag = "AT+CMGS=")]
    SendSms(#[parser(parse_raw_data)] &'a [u8]),
    /// 3GPP TS 27.005: Write message to memory
    #[command(tag = "AT+CMGW=")]
    StoreSms(u8),
    /// 3GPP TS 27.005: Read message
    #[command(tag = "AT+CMGR=")]
    ReadSms(u8),
    /// 3GPP TS 27.005: Delete SMS Message
    #[command(tag = "AT+CMGD=")]
    DeleteSms(u8),
    /// 3GPP TS 27.005: New message acknowledgement with value (e.g. AT+CNMA=1)
    #[command(tag = "AT+CNMA=")]
    SendSmsAckWithVal(u8),
    /// 3GPP TS 27.005: New message acknowledgement
    #[command(tag = "AT+CNMA")]
    SendSmsAck,
    /// Query STK ready
    #[command(tag = "AT+CUSATD?")]
    QueryStkReady,
    /// Send STK envelope
    #[command(tag = "AT+CUSATE=")]
    SendStkEnvelope(QuotedString<'a>),
    /// Facility lock
    #[command(tag = "AT+CLCK=")]
    SetFacilityLock(QuotedString<'a>, u8, Option<QuotedString<'a>>, Option<u8>),
    /// Call forwarding
    #[command(tag = "AT+CCFC=")]
    CallForwarding {
        reason: u8,
        mode: u8,
        number: Option<QuotedString<'a>>,
        r#type: Option<u8>,
        class: Option<u8>,
        subaddr: Option<QuotedString<'a>>,
        satype: Option<u8>,
        time: Option<u8>,
    },
    /// Calling line identification restriction
    #[command(tag = "AT+CLIR?")]
    QueryClir,
    /// Non-standard Goldfish CLIR syntax used by Goldfish guest Radio HAL
    /// (device/generic/goldfish/hals/radio/RadioVoice.cpp: RadioVoice::setClir
    /// formats request as "AT+CLIR: <status>")
    #[command(tag = "AT+CLIR: ")]
    SetClirGoldfish(u8),
    #[command(tag = "AT+CLIR=")]
    SetClir(u8),
    /// Calling line identification presentation
    #[command(tag = "AT+CLIP=")]
    SetClip(u8),
    /// Query Calling line identification presentation
    #[command(tag = "AT+CLIP?")]
    QueryClip,
    /// Configure call mode
    #[command(tag = "AT+CMOD=")]
    SetCallMode(u8),
    /// Connected line identification presentation
    #[command(tag = "AT+COLP=")]
    SetColp(u8),
    /// Select TE character set
    #[command(tag = "AT+CSCS=")]
    SetCharacterSet(QuotedString<'a>),
    /// Call waiting
    #[command(tag = "AT+CCWA=")]
    SetCallWaiting(u8, Option<u8>, Option<u8>),
    /// Supplementary service notification
    #[command(tag = "AT+CSSN=")]
    SuppServiceNotification(u8, u8),
    /// Unstructured Supplementary Service Data
    #[command(tag = "AT+CUSD=")]
    SetUssd { mode: u8, message: Option<QuotedString<'a>>, dcs: Option<u8> },
    /// Define PDP context
    #[command(tag = "AT+CGDCONT=")]
    DefinePdpContext(
        u8,
        QuotedString<'a>,
        QuotedString<'a>,
        Option<QuotedString<'a>>,
        Option<u8>,
        Option<u8>,
    ),
    /// Read PDP context
    #[command(tag = "AT+CGDCONT?")]
    QueryPdpContext,
    /// Quality of service profile (minimum)
    #[command(tag = "AT+CGEQMIN=")]
    SetQualityOfServiceMinimum(u8, u8, u8, u8, u8, u8),
    /// Quality of service profile (minimum)
    #[command(tag = "AT+CGEQMIN?")]
    QueryQualityOfServiceMinimum,
    /// Quality of service profile (requested)
    #[command(tag = "AT+CGEQREQ=")]
    SetQualityOfServiceRequested(u8, u8, u8, u8, u8, u8),
    /// Quality of service profile (requested)
    #[command(tag = "AT+CGEQREQ?")]
    QueryQualityOfServiceRequested,
    /// Quality of service profile (minimum) GPRS
    #[command(tag = "AT+CGQMIN=")]
    SetQualityOfServiceMinimumGprs(u8, u8, u8, u8, u8, u8),
    /// Quality of service profile (minimum) GPRS
    #[command(tag = "AT+CGQMIN?")]
    QueryQualityOfServiceMinimumGprs,
    /// Quality of service profile (requested) GPRS
    #[command(tag = "AT+CGQREQ=")]
    SetQualityOfServiceRequestedGprs(u8, u8, u8, u8, u8, u8),
    /// Quality of service profile (requested) GPRS
    #[command(tag = "AT+CGQREQ?")]
    QueryQualityOfServiceRequestedGprs,
    /// PDP context activate
    #[command(tag = "AT+CGACT=")]
    SetPdpContextActivate(u8, u8),
    /// Query PDP context activate status
    #[command(tag = "AT+CGACT?")]
    QueryPdpContextActivate,
    /// PS attach or detach
    #[command(tag = "AT+CGATT=")]
    SetPsAttach(u8),
    /// Query PS attach status
    #[command(tag = "AT+CGATT?")]
    QueryPsAttach,
    /// PDP context modify
    #[command(tag = "AT+CGCMOD=")]
    SetPdpContextModify(u8),
    /// Enter data state
    #[command(tag = "AT+CGDATA=")]
    EnterDataState(u8),
    /// Query current network technology
    #[command(tag = "AT+CTEC?")]
    QueryCurrentNetworkTechnology,
    /// Query supported network technology
    #[command(tag = "AT+CTEC=?")]
    QuerySupportedNetworkTechnology,
    /// Set network technology
    #[command(tag = "AT+CTEC=")]
    SetNetworkTechnology(u8, #[parser(parse_raw_data)] &'a [u8]),
    /// Packet event reporting
    #[command(tag = "AT+CGEREP=")]
    SetPacketEventReporting(u8, u8),
    /// Show PDP address
    #[command(tag = "AT+CGPADDR=")]
    ShowPdpAddress(u8),
    /// Read dynamic parameters
    #[command(tag = "AT+CGCONTRDP=")]
    ReadDynamicParam(u8),
    /// 3GPP TS 27.005: Set SMS message format
    #[command(tag = "AT+CMGF=")]
    SetSmsMessageFormat(u8),
    /// 3GPP TS 27.005: Set preferred message storage
    #[command(tag = "AT+CPMS=")]
    SetPreferredMessageStorage(QuotedString<'a>, QuotedString<'a>, QuotedString<'a>),
    /// 3GPP TS 27.005: Query preferred message storage
    #[command(tag = "AT+CPMS?")]
    QueryPreferredMessageStorage,
    /// 3GPP TS 27.005: Set broadcast config
    #[command(tag = "AT+CSCB=")]
    BroadcastConfig(u8, QuotedString<'a>, QuotedString<'a>),
    /// 3GPP TS 27.005: Query broadcast config
    #[command(tag = "AT+CSCB?")]
    QueryBroadcastConfig,
    /// 3GPP TS 27.005: Set SMSC address
    #[command(tag = "AT+CSCA=")]
    SetSmscAddress(QuotedString<'a>),
    /// 3GPP TS 27.005: Get SMSC address
    #[command(tag = "AT+CSCA?")]
    GetSmscAddress,
    /// VENDOR: Remote SMS
    #[command(tag = "AT+REMOTESMS=")]
    RemoteSms(QuotedString<'a>),
    /// Set STK
    #[command(tag = "AT+STK=")]
    SetStk(u8),
    /// Set STK enabled
    #[command(tag = "AT+STKEN=")]
    SetStkEnabled(u8),
    /// Set STK unsolicited result
    #[command(tag = "AT+STKUR=")]
    SetStkUnsolicitedResult(u8),
    /// Report mobile equipment error
    #[command(tag = "AT+CMEE=")]
    SetReportMobileEquipmentError(u8),
    /// Query report mobile equipment error
    #[command(tag = "AT+CMEE?")]
    QueryReportMobileEquipmentError,
    /// Query supported report mobile equipment error modes
    #[command(tag = "AT+CMEE=?")]
    QuerySupportedReportMobileEquipmentError,
    /// Goldfish specific concatenated init command
    #[command(tag = "ATE0Q0V1")]
    GoldfishInitSequence,
    /// Set echo
    #[command(tag = "ATE")]
    SetEcho(u8),
    /// Set speaker volume
    #[command(tag = "ATL")]
    SetSpeakerVolume(u8),
    /// Set speaker mute
    #[command(tag = "ATM")]
    SetSpeakerMute(u8),
    /// Set quiet mode
    #[command(tag = "ATQ")]
    SetQuietMode(u8),
    /// Set verbose mode
    #[command(tag = "ATV")]
    SetVerboseMode(u8),
    /// Reset to factory defaults
    #[command(tag = "AT&F")]
    ResetToFactoryDefaults,
    /// View active configuration
    #[command(tag = "AT&V")]
    ViewActiveConfiguration,
    /// Write active configuration
    #[command(tag = "AT&W")]
    WriteActiveConfiguration,
    /// Reset
    #[command(tag = "ATZ")]
    Reset,
    /// Get identification information
    #[command(tag = "ATI")]
    GetIdentificationInformation,
    /// Set auto-answer
    #[command(tag = "ATS0=")]
    SetAutoAnswer(u8),
    /// Set command termination character
    #[command(tag = "ATS3=")]
    SetCommandTerminationCharacter(u8),
    /// Set response formatting character
    #[command(tag = "ATS4=")]
    SetResponseFormattingCharacter(u8),
    /// Set command line editing character
    #[command(tag = "ATS5=")]
    SetCommandLineEditingCharacter(u8),
    /// Set pause before blind dialing
    #[command(tag = "ATS6=")]
    SetPauseBeforeBlindDialing(u8),
    /// Set connection completion timeout
    #[command(tag = "ATS7=")]
    SetConnectionCompletionTimeout(u8),
    /// Set comma dial modifier time
    #[command(tag = "ATS8=")]
    SetCommaDialModifierTime(u8),
    /// Set automatic disconnect delay
    #[command(tag = "ATS10=")]
    SetAutomaticDisconnectDelay(u8),
    /// Get capabilities
    #[command(tag = "AT+GCAP")]
    GetCapabilities,
    /// Get manufacturer identification
    #[command(tag = "AT+GMI")]
    GetManufacturerIdentification,
    /// Get model identification
    #[command(tag = "AT+GMM")]
    GetModelId,
    /// Get revision identification
    #[command(tag = "AT+GMR")]
    GetRevision,
    /// Get serial number
    #[command(tag = "AT+GSN")]
    GetSerialNumber,
    /// Get product serial number with type
    #[command(tag = "AT+CGSN=")]
    GetProductSerialNumberGsmWithType(u8),
    /// Get product serial number (IMEI)
    #[command(tag = "AT+CGSN")]
    GetProductSerialNumberGsm,
    /// Set TE-TA control character framing
    #[command(tag = "AT+ICF=")]
    SetTeTaControlCharacterFraming(u8, u8),
    /// Set TE-TA local data flow control
    #[command(tag = "AT+IFC=")]
    SetTeTaLocalDataFlowControl(u8, u8),
    /// Set TE-TA fixed local rate
    #[command(tag = "AT+IPR=")]
    SetTeTaFixedLocalRate(u32),
    /// Set clock
    #[command(tag = "AT+CCLK=")]
    SetTime(QuotedString<'a>),
    /// Query clock
    #[command(tag = "AT+CCLK?")]
    QueryTime,
    /// Test command
    #[command(tag = "AT")]
    Test,
}

impl<'a> Command<'a> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cpin_query() {
        let (rem, cmd) = Command::parse(b"AT+CPIN?").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::GetSimStatus);
    }

    #[test]
    fn test_parse_ipr() {
        let (rem, cmd) = Command::parse(b"AT+IPR=9600").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetTeTaFixedLocalRate(9600));
    }

    #[test]
    fn test_parse_cmee() {
        let (rem, cmd) = Command::parse(b"AT+CMEE=1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetReportMobileEquipmentError(1));

        let (rem, cmd) = Command::parse(b"AT+CMEE?").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::QueryReportMobileEquipmentError);

        let (rem, cmd) = Command::parse(b"AT+CMEE=?").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::QuerySupportedReportMobileEquipmentError);
    }

    #[test]
    fn test_parse_cgdcont() {
        let (rem, cmd) = Command::parse(b"AT+CGDCONT=1,\"IP\",\"apn\"").unwrap();
        assert!(rem.is_empty());
        assert_eq!(
            cmd,
            Command::DefinePdpContext(
                1,
                QuotedString(b"IP"),
                QuotedString(b"apn"),
                None,
                None,
                None
            )
        );
    }

    #[test]
    fn test_parse_cgdcont_extra() {
        let (rem, cmd) =
            Command::parse(b"AT+CGDCONT=1,\"IPV6\",\"fast.t-mobile.com\",,0,0").unwrap();
        assert!(rem.is_empty());
        assert_eq!(
            cmd,
            Command::DefinePdpContext(
                1,
                QuotedString(b"IPV6"),
                QuotedString(b"fast.t-mobile.com"),
                None,
                Some(0),
                Some(0)
            )
        );
    }

    #[test]
    fn test_parse_cgeqmin() {
        let (rem, cmd) = Command::parse(b"AT+CGEQMIN=1,2,3,4,5,6").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetQualityOfServiceMinimum(1, 2, 3, 4, 5, 6));
    }

    #[test]
    fn test_parse_cgeqreq() {
        let (rem, cmd) = Command::parse(b"AT+CGEQREQ=1,2,3,4,5,6").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetQualityOfServiceRequested(1, 2, 3, 4, 5, 6));
    }

    #[test]
    fn test_parse_cgqmin() {
        let (rem, cmd) = Command::parse(b"AT+CGQMIN=1,2,3,4,5,6").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetQualityOfServiceMinimumGprs(1, 2, 3, 4, 5, 6));
    }

    #[test]
    fn test_parse_cgqreq() {
        let (rem, cmd) = Command::parse(b"AT+CGQREQ=1,2,3,4,5,6").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetQualityOfServiceRequestedGprs(1, 2, 3, 4, 5, 6));
    }

    #[test]
    fn test_parse_cgact() {
        let (rem, cmd) = Command::parse(b"AT+CGACT=1,1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetPdpContextActivate(1, 1));
    }

    #[test]
    fn test_parse_cgatt() {
        let (rem, cmd) = Command::parse(b"AT+CGATT=1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetPsAttach(1));
    }

    #[test]
    fn test_parse_cgcmod() {
        let (rem, cmd) = Command::parse(b"AT+CGCMOD=1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetPdpContextModify(1));
    }

    #[test]
    fn test_parse_cgdata() {
        let (rem, cmd) = Command::parse(b"AT+CGDATA=1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::EnterDataState(1));
    }

    #[test]
    fn test_parse_cgerep() {
        let (rem, cmd) = Command::parse(b"AT+CGEREP=1,1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetPacketEventReporting(1, 1));
    }

    #[test]
    fn test_parse_cgpaddr() {
        let (rem, cmd) = Command::parse(b"AT+CGPADDR=1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::ShowPdpAddress(1));
    }

    #[test]
    fn test_parse_cmgf() {
        let (rem, cmd) = Command::parse(b"AT+CMGF=1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetSmsMessageFormat(1));
    }

    #[test]
    fn test_parse_stk() {
        let (rem, cmd) = Command::parse(b"AT+STK=1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetStk(1));
    }

    #[test]
    fn test_parse_stken() {
        let (rem, cmd) = Command::parse(b"AT+STKEN=1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetStkEnabled(1));
    }

    #[test]
    fn test_parse_stkur() {
        let (rem, cmd) = Command::parse(b"AT+STKUR=1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetStkUnsolicitedResult(1));
    }

    #[test]
    fn test_parse_crsm() {
        let (rem, cmd) = Command::parse(b"AT+CRSM=176,28480,0,0,7").unwrap();
        assert!(rem.is_empty());
        assert_eq!(
            cmd,
            Command::SimIo { command: 176, file_id: 28480, p1: 0, p2: 0, p3: 7, data: None }
        );
    }

    #[test]
    fn test_parse_gprs_dial() {
        let (rem, cmd) = Command::parse(b"ATD*99***1#\r\n").unwrap();
        assert_eq!(rem, b"\r\n");
        assert_eq!(cmd, Command::Dial(b"*99***1#"));
    }

    #[test]
    fn test_parse_ccwa_set_single_arg() {
        let (rem, cmd) = Command::parse(b"AT+CCWA=1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetCallWaiting(1, None, None));
    }

    #[test]
    fn test_parse_ccwa_set_multiple_args() {
        let (rem, cmd) = Command::parse(b"AT+CCWA=1,2,7").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetCallWaiting(1, Some(2), Some(7)));
    }

    #[test]
    fn test_parse_cmod_set() {
        let (rem, cmd) = Command::parse(b"AT+CMOD=0").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetCallMode(0));
    }

    #[test]
    fn test_parse_colp_set() {
        let (rem, cmd) = Command::parse(b"AT+COLP=0").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetColp(0));
    }

    #[test]
    fn test_parse_cscs_set() {
        let (rem, cmd) = Command::parse(br#"AT+CSCS="HEX""#).unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetCharacterSet(QuotedString(b"HEX")));
    }

    #[test]
    fn test_parse_clir_set_goldfish() {
        let (rem, cmd) = Command::parse(b"AT+CLIR: 0").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetClirGoldfish(0));
    }

    #[test]
    fn test_parse_clir_set_standard() {
        let (rem, cmd) = Command::parse(b"AT+CLIR=0").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetClir(0));
    }

    #[test]
    fn test_parse_cgsn_query() {
        let (rem, cmd) = Command::parse(b"AT+CGSN").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::GetProductSerialNumberGsm);
    }

    #[test]
    fn test_parse_cgsn_set() {
        let (rem, cmd) = Command::parse(b"AT+CGSN=2").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::GetProductSerialNumberGsmWithType(2));
    }

    #[test]
    fn test_parse_cops_set() {
        let (rem, cmd) = Command::parse(b"AT+COPS=3,2").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::SetOperator { mode: 3, format: Some(2), oper: None });
    }

    #[test]
    fn test_parse_at() {
        let (rem, cmd) = Command::parse(b"AT").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Test);
    }

    #[test]
    fn test_parse_at_invalid() {
        let (rem, cmd) = Command::parse(b"AT+INVALID").unwrap();
        assert!(!rem.is_empty());
        assert_eq!(cmd, Command::Test);
    }
}
