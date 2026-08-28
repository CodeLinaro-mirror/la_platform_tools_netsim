// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use nom::{IResult, bytes::complete::tag};

use crate::{
    call_service::CallCommand, data_service::DataCommand, misc_service::MiscCommand,
    network_service::NetworkCommand, sim_service::SimCommand, sms_service::SmsCommand,
    stk_service::StkCommand, sup_service::SupCommand, types::Parsable,
};

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

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct PinString<'a>(pub &'a [u8]);

impl<'a> PinString<'a> {
    pub fn to_vec(self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl<'a> AsRef<[u8]> for PinString<'a> {
    fn as_ref(&self) -> &[u8] {
        self.0
    }
}

impl<'a> std::ops::Deref for PinString<'a> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'a> Parsable<'a> for PinString<'a> {
    fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self> {
        if let Ok((rem, quoted)) = QuotedString::parse(input) {
            return Ok((rem, PinString(quoted.0)));
        }
        use nom::bytes::complete::take_while1;
        let (input, content) =
            take_while1(|c: u8| c != b',' && c != b';' && c != b'\r' && c != b'\n')(input)?;
        Ok((input, PinString(content)))
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct ApduData<'a>(pub &'a [u8]);

impl<'a> ApduData<'a> {
    pub fn to_vec(self) -> Vec<u8> {
        self.0.to_vec()
    }
}

impl<'a> AsRef<[u8]> for ApduData<'a> {
    fn as_ref(&self) -> &[u8] {
        self.0
    }
}

impl<'a> Parsable<'a> for ApduData<'a> {
    fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self> {
        use nom::{
            branch::alt,
            bytes::complete::{take_while, take_while1},
            sequence::delimited,
        };
        let parse_quoted = delimited(tag(br#"""#), take_while(|c| c != b'"'), tag(br#"""#));
        let parse_unquoted = take_while1(|c: u8| c.is_ascii_hexdigit());
        let (input, content) = alt((parse_quoted, parse_unquoted))(input)?;
        Ok((input, ApduData(content)))
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

/// Top-level AT Command wrapper enum across all modem-rs services.
#[derive(Debug, PartialEq, Clone)]
pub enum Command<'a> {
    Sim(SimCommand<'a>),
    Call(CallCommand<'a>),
    Sms(SmsCommand<'a>),
    Network(NetworkCommand<'a>),
    Data(DataCommand<'a>),
    Misc(MiscCommand<'a>),
    Sup(SupCommand<'a>),
    Stk(StkCommand<'a>),
}

impl<'a> Command<'a> {
    pub fn parse(input: &'a [u8]) -> IResult<&'a [u8], Self> {
        use nom::{branch::alt, combinator::map};
        alt((
            map(SimCommand::parse, Self::Sim),
            map(CallCommand::parse, Self::Call),
            map(SmsCommand::parse, Self::Sms),
            map(NetworkCommand::parse, Self::Network),
            map(DataCommand::parse, Self::Data),
            map(SupCommand::parse, Self::Sup),
            map(StkCommand::parse, Self::Stk),
            map(MiscCommand::parse, Self::Misc),
        ))(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        CallMode, CallWaitingMode, CallWaitingPresentation, ClirMode, CopsFormat, CopsMode,
        DialArgs, PdpType, PhoneNumber, ProductSerialNumberType,
    };

    #[test]
    fn test_parse_cpin_query() {
        let (rem, cmd) = Command::parse(b"AT+CPIN?").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Sim(SimCommand::GetSimStatus));
    }

    #[test]
    fn test_parse_ipr() {
        let (rem, cmd) = Command::parse(b"AT+IPR=9600").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Misc(MiscCommand::SetTeTaFixedLocalRate(9600)));
    }

    #[test]
    fn test_parse_cmee() {
        let (rem, cmd) = Command::parse(b"AT+CMEE=1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(
            cmd,
            Command::Misc(MiscCommand::SetReportMobileEquipmentError(
                crate::types::CmeeMode::Numeric
            ))
        );

        let (rem, cmd) = Command::parse(b"AT+CMEE?").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Misc(MiscCommand::QueryReportMobileEquipmentError));

        let (rem, cmd) = Command::parse(b"AT+CMEE=?").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Misc(MiscCommand::QuerySupportedReportMobileEquipmentError));
    }

    #[test]
    fn test_parse_cgdcont() {
        let (rem, cmd) = Command::parse(b"AT+CGDCONT=1,\"IP\",\"apn\"").unwrap();
        assert!(rem.is_empty());
        assert_eq!(
            cmd,
            Command::Data(DataCommand::DefinePdpContext(
                1,
                PdpType::Ip,
                QuotedString(b"apn"),
                None,
                None,
                None
            ))
        );
    }

    #[test]
    fn test_parse_cgdcont_extra() {
        let (rem, cmd) =
            Command::parse(b"AT+CGDCONT=1,\"IPV6\",\"fast.t-mobile.com\",,0,0").unwrap();
        assert!(rem.is_empty());
        assert_eq!(
            cmd,
            Command::Data(DataCommand::DefinePdpContext(
                1,
                PdpType::Ipv6,
                QuotedString(b"fast.t-mobile.com"),
                None,
                Some(0),
                Some(0)
            ))
        );
    }

    #[test]
    fn test_parse_cgeqmin() {
        let (rem, cmd) = Command::parse(b"AT+CGEQMIN=1,2,3,4,5,6").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Data(DataCommand::SetQualityOfServiceMinimum(1, 2, 3, 4, 5, 6)));
    }

    #[test]
    fn test_parse_cgeqreq() {
        let (rem, cmd) = Command::parse(b"AT+CGEQREQ=1,2,3,4,5,6").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Data(DataCommand::SetQualityOfServiceRequested(1, 2, 3, 4, 5, 6)));
    }

    #[test]
    fn test_parse_cgqmin() {
        let (rem, cmd) = Command::parse(b"AT+CGQMIN=1,2,3,4,5,6").unwrap();
        assert!(rem.is_empty());
        assert_eq!(
            cmd,
            Command::Data(DataCommand::SetQualityOfServiceMinimumGprs(1, 2, 3, 4, 5, 6))
        );
    }

    #[test]
    fn test_parse_cgqreq() {
        let (rem, cmd) = Command::parse(b"AT+CGQREQ=1,2,3,4,5,6").unwrap();
        assert!(rem.is_empty());
        assert_eq!(
            cmd,
            Command::Data(DataCommand::SetQualityOfServiceRequestedGprs(1, 2, 3, 4, 5, 6))
        );
    }

    #[test]
    fn test_parse_cgact() {
        let (rem, cmd) = Command::parse(b"AT+CGACT=1,1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Data(DataCommand::SetPdpContextActivate(1, 1)));
    }

    #[test]
    fn test_parse_cgatt() {
        let (rem, cmd) = Command::parse(b"AT+CGATT=1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Data(DataCommand::SetPsAttach(1)));
    }

    #[test]
    fn test_parse_cgcmod() {
        let (rem, cmd) = Command::parse(b"AT+CGCMOD=1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Data(DataCommand::SetPdpContextModify(1)));
    }

    #[test]
    fn test_parse_cgdata() {
        let (rem, cmd) = Command::parse(b"AT+CGDATA=1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Data(DataCommand::EnterDataState(1)));
    }

    #[test]
    fn test_parse_cgerep() {
        let (rem, cmd) = Command::parse(b"AT+CGEREP=1,1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Data(DataCommand::SetPacketEventReporting(1, 1)));
    }

    #[test]
    fn test_parse_cgpaddr() {
        let (rem, cmd) = Command::parse(b"AT+CGPADDR=1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Data(DataCommand::ShowPdpAddress(1)));
    }

    #[test]
    fn test_parse_cmgf() {
        let (rem, cmd) = Command::parse(b"AT+CMGF=1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(
            cmd,
            Command::Sms(SmsCommand::SetSmsMessageFormat(crate::sms_service::MessageFormat::Text))
        );
    }

    #[test]
    fn test_parse_stk() {
        let (rem, cmd) = Command::parse(b"AT+STK=1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Stk(StkCommand::SetStk(true)));
    }

    #[test]
    fn test_parse_stken() {
        let (rem, cmd) = Command::parse(b"AT+STKEN=1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Stk(StkCommand::SetStkEnabled(true)));
    }

    #[test]
    fn test_parse_stkur() {
        let (rem, cmd) = Command::parse(b"AT+STKUR=1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Stk(StkCommand::SetStkUnsolicitedResult(true)));
    }

    #[test]
    fn test_parse_crsm() {
        let (rem, cmd) = Command::parse(b"AT+CRSM=176,28480,0,0,7").unwrap();
        assert!(rem.is_empty());
        assert_eq!(
            cmd,
            Command::Sim(SimCommand::SimIo {
                command: 176,
                file_id: 28480,
                p1: 0,
                p2: 0,
                p3: 7,
                data: None,
                path: None,
            })
        );

        let (rem2, cmd2) = Command::parse(b"AT+CRSM=176,28480,0,0,7,,").unwrap();
        assert!(rem2.is_empty());
        assert_eq!(
            cmd2,
            Command::Sim(SimCommand::SimIo {
                command: 176,
                file_id: 28480,
                p1: 0,
                p2: 0,
                p3: 7,
                data: None,
                path: None,
            })
        );
    }

    #[test]
    fn test_parse_gprs_dial() {
        let (rem, cmd) = Command::parse(b"ATD*99***1#\r\n").unwrap();
        assert_eq!(rem, b"\r\n");
        let expected_dial_args = DialArgs {
            number: PhoneNumber::new("*99***1#"),
            clir: ClirMode::SubscriptionDefault,
            is_emergency: false,
        };
        assert_eq!(cmd, Command::Call(CallCommand::Dial(expected_dial_args)));
    }

    #[test]
    fn test_parse_ccwa_set_single_arg() {
        let (rem, cmd) = Command::parse(b"AT+CCWA=1").unwrap();
        assert!(rem.is_empty());
        assert_eq!(
            cmd,
            Command::Sup(SupCommand::SetCallWaiting(CallWaitingPresentation::Enable, None, None))
        );
    }

    #[test]
    fn test_parse_ccwa_set_multiple_args() {
        let (rem, cmd) = Command::parse(b"AT+CCWA=1,2,7").unwrap();
        assert!(rem.is_empty());
        assert_eq!(
            cmd,
            Command::Sup(SupCommand::SetCallWaiting(
                CallWaitingPresentation::Enable,
                Some(CallWaitingMode::Query),
                Some(7)
            ))
        );
    }

    #[test]
    fn test_parse_cmod_set() {
        let (rem, cmd) = Command::parse(b"AT+CMOD=0").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Misc(MiscCommand::SetCallMode(CallMode::SingleMode)));
    }

    #[test]
    fn test_parse_colp_set() {
        let (rem, cmd) = Command::parse(b"AT+COLP=0").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Sup(SupCommand::SetColp(0)));
    }

    #[test]
    fn test_parse_cscs_set() {
        let (rem, cmd) = Command::parse(br#"AT+CSCS="HEX""#).unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Misc(MiscCommand::SetCharacterSet(QuotedString(b"HEX"))));
    }

    #[test]
    fn test_parse_clir_set_goldfish() {
        let (rem, cmd) = Command::parse(b"AT+CLIR: 0").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Sup(SupCommand::SetClirGoldfish(ClirMode::SubscriptionDefault)));
    }

    #[test]
    fn test_parse_clir_set_standard() {
        let (rem, cmd) = Command::parse(b"AT+CLIR=0").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Sup(SupCommand::SetClir(ClirMode::SubscriptionDefault)));
    }

    #[test]
    fn test_parse_cgsn_query() {
        let (rem, cmd) = Command::parse(b"AT+CGSN").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Misc(MiscCommand::GetProductSerialNumberGsm));
    }

    #[test]
    fn test_parse_cgsn_set() {
        let (rem, cmd) = Command::parse(b"AT+CGSN=2").unwrap();
        assert!(rem.is_empty());
        assert_eq!(
            cmd,
            Command::Misc(MiscCommand::GetProductSerialNumberGsmWithType(
                ProductSerialNumberType::ImeiWithSvn
            ))
        );
    }

    #[test]
    fn test_parse_cops_set() {
        let (rem, cmd) = Command::parse(b"AT+COPS=3,2").unwrap();
        assert!(rem.is_empty());
        assert_eq!(
            cmd,
            Command::Network(NetworkCommand::SetOperator {
                mode: CopsMode::SetFormatOnly,
                format: Some(CopsFormat::Numeric),
                oper: None,
            })
        );
    }

    #[test]
    fn test_parse_at() {
        let (rem, cmd) = Command::parse(b"AT").unwrap();
        assert!(rem.is_empty());
        assert_eq!(cmd, Command::Misc(MiscCommand::Test));
    }

    #[test]
    fn test_parse_at_invalid() {
        let (rem, cmd) = Command::parse(b"AT+INVALID").unwrap();
        assert!(!rem.is_empty());
        assert_eq!(cmd, Command::Misc(MiscCommand::Test));
    }
}
