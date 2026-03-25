use ap_actor::shared::SharedKeyStore;
use netsim_packets::{
    ieee80211::MacAddress,
    netlink::{
        hwsim_attr_set::HwsimAttrSet,
        hwsim_frame::HwsimFrame,
        mac80211_hwsim::{TxRate, TxRateFlag},
        HwsimCmd, HwsimMsg, HwsimMsgHdr, NlMsgHdr,
    },
};

use crate::{error::WifiError, medium::WifiResult};

pub const RX_RATE: u32 = 1;
pub const SIGNAL: u32 = 4294967246; // -50
const NL_MSG_HDR_LEN: usize = 16;
const NLMSG_MIN_TYPE: u16 = 0x10;
const NL_AUTO_SEQ: u16 = 0;
const NL_AUTO_PORT: u32 = 0;

pub fn create_encrypted_hwsim_msg(
    frame: &HwsimFrame,
    ieee80211: &netsim_packets::ieee80211::Ieee80211,
    dest_hwsim_addr: &MacAddress,
    key_store: &SharedKeyStore,
    simulate_ap_reflection: bool,
) -> WifiResult<HwsimMsg> {
    let attrs = &frame.attrs;
    let ap_bssid = key_store.get_bssid();

    let ieee80211_response = match simulate_ap_reflection
        && ieee80211.is_to_ap()
        && Some(ieee80211.get_bssid().unwrap_or(MacAddress::new([0, 0, 0, 0, 0, 0]))) == ap_bssid
    {
        true => std::borrow::Cow::Owned(
            ieee80211
                .into_from_ap()
                .map_err(|e| WifiError::Frame(format!("{e}")))?
                .try_into()
                .map_err(|e| WifiError::Frame(format!("{e}")))?,
        ),
        false => std::borrow::Cow::Borrowed(ieee80211),
    };

    let ieee80211_response = if let Some(encrypted_bytes) =
        key_store.try_encrypt(&ieee80211_response)
    {
        std::borrow::Cow::Owned(
            netsim_packets::ieee80211::Ieee80211::decode(&encrypted_bytes)
                .map_err(|e| WifiError::Frame(format!("Failed to decode encrypted frame: {e}")))?,
        )
    } else {
        ieee80211_response
    };
    let frame_bytes = ieee80211_response.as_bytes();

    let hwsim_msg = construct_hwsim_msg(
        &dest_hwsim_addr.bytes,
        frame_bytes,
        None, // Legacy did not include transmitter for RX frames
        attrs.freq,
        Some(attrs.rx_rate_idx.unwrap_or(RX_RATE)),
        Some(attrs.signal.unwrap_or(SIGNAL)),
        attrs.flags,
        None, // Do not echo the TX cookie to the RX path
        attrs.tx_info.as_deref(),
        attrs.tx_info_flags.as_deref(),
        frame.hwsim_msg.nl_hdr.nlmsg_flags,
    )?;

    Ok(hwsim_msg)
}

pub fn parse_hwsim_frame(packet: &bytes::Bytes, client_id: u32) -> WifiResult<HwsimFrame> {
    let hwsim_msg = HwsimMsg::decode_full(packet).map_err(|e| WifiError::Frame(e.to_string()))?;
    match hwsim_msg.hwsim_hdr.hwsim_cmd {
        HwsimCmd::Frame => {
            let frame =
                HwsimFrame::parse(&hwsim_msg).map_err(|e| WifiError::Frame(e.to_string()))?;
            if frame.transmitter.is_none()
                || frame.flags.is_none()
                || frame.cookie.is_none()
                || frame.tx_info.is_none()
            {
                return Err(WifiError::Frame(format!(
                    "Missing Hwsim attributes for incoming packet for client: {client_id}"
                )));
            }
            let _ = frame.transmitter.ok_or(WifiError::Frame(format!(
                "Missing transmitter attribute in frame for client: {client_id}"
            )))?;
            let _ = frame.flags.ok_or(WifiError::Frame(format!(
                "Missing flags attribute in frame for client: {client_id}"
            )))?;
            let _ = frame.cookie.ok_or(WifiError::Frame(format!(
                "Missing cookie attribute in frame for client: {client_id}"
            )))?;
            // Removed debug logging here to avoid log dependency in utils, or we can add
            // it. Caller can log if needed.
            Ok(frame)
        }
        _ => Err(WifiError::Frame(format!(
            "Another command found {hwsim_msg:?} for client: {client_id}"
        ))),
    }
}

pub fn create_hwsim_msg_from_frame(
    ieee80211: &netsim_packets::ieee80211::Ieee80211,
    dest_hwsim_addr: &netsim_packets::ieee80211::MacAddress,
    freq: u32,
    transmitter: Option<&netsim_packets::ieee80211::MacAddress>,
) -> WifiResult<HwsimMsg> {
    let frame_bytes = ieee80211.as_bytes();
    construct_hwsim_msg(
        &dest_hwsim_addr.bytes,
        frame_bytes,
        transmitter.map(|t| &t.bytes),
        Some(freq),
        Some(RX_RATE),
        Some(SIGNAL),
        Some(0),   // flags
        Some(0),   // cookie
        Some(&[]), // tx_info
        None,      // tx_info_flags
        NL_AUTO_SEQ,
    )
}

fn construct_hwsim_msg(
    receiver: &[u8; 6],
    frame: &[u8],
    transmitter: Option<&[u8; 6]>,
    freq: Option<u32>,
    rx_rate: Option<u32>,
    signal: Option<u32>,
    flags: Option<u32>,
    cookie: Option<u64>,
    tx_info: Option<&[TxRate]>,
    tx_info_flags: Option<&[TxRateFlag]>,
    nlmsg_flags: u16,
) -> WifiResult<HwsimMsg> {
    let mut builder = HwsimAttrSet::builder();
    builder.receiver(receiver);
    builder.frame(frame);
    if let Some(tx) = transmitter {
        builder.transmitter(tx);
    }
    if let Some(r) = rx_rate {
        builder.rx_rate(r);
    }
    if let Some(s) = signal {
        builder.signal(s);
    }
    if let Some(f) = freq {
        builder.freq(f);
    }
    if let Some(f) = flags {
        builder.flags(f);
    }
    if let Some(c) = cookie {
        builder.cookie(c);
    }
    if let Some(t) = tx_info {
        builder.tx_info(t);
    }
    if let Some(t) = tx_info_flags {
        builder.tx_info_flags(t);
    }

    let attributes = builder.build().map_err(|e| WifiError::Frame(e.to_string()))?.attributes;
    let hwsim_hdr = HwsimMsgHdr { hwsim_cmd: HwsimCmd::Frame, hwsim_version: 0, reserved: 0 };
    let nlmsg_len = (NL_MSG_HDR_LEN + hwsim_hdr.encoded_len() as usize + attributes.len()) as u32;
    let nl_hdr = NlMsgHdr {
        nlmsg_len,
        nlmsg_type: NLMSG_MIN_TYPE,
        nlmsg_flags,
        nlmsg_seq: NL_AUTO_PORT,
        nlmsg_pid: 0,
    };
    Ok(HwsimMsg { nl_hdr, hwsim_hdr, attributes })
}

pub fn build_tx_info(hwsim_msg: &HwsimMsg) -> WifiResult<HwsimMsg> {
    let attrs =
        HwsimAttrSet::parse(&hwsim_msg.attributes).map_err(|e| WifiError::Frame(e.to_string()))?;

    let hwsim_hdr = &hwsim_msg.hwsim_hdr;
    let nl_hdr = &hwsim_msg.nl_hdr;
    let mut new_attr_builder = HwsimAttrSet::builder();
    const HWSIM_TX_STAT_ACK: u32 = 1 << 2;

    new_attr_builder
        .transmitter(
            &attrs
                .transmitter
                .ok_or(WifiError::Frame("Missing transmitter in HwsimAttrSet".into()))?
                .into(),
        )
        .flags(
            attrs.flags.ok_or(WifiError::Frame("Missing flags in HwsimAttrSet".into()))?
                | HWSIM_TX_STAT_ACK,
        )
        .cookie(attrs.cookie.ok_or(WifiError::Frame("Missing cookie in HwsimAttrSet".into()))?)
        .signal(attrs.signal.unwrap_or(SIGNAL))
        .tx_info(
            attrs
                .tx_info
                .ok_or(WifiError::Frame("Missing tx_info in HwsimAttrSet".into()))?
                .as_slice(),
        );

    let new_attr = new_attr_builder.build().map_err(|e| WifiError::Frame(e.to_string()))?;
    let nlmsg_len = (NlMsgHdr::SIZE + HwsimMsgHdr::SIZE + new_attr.attributes.len()) as u32;
    let new_hwsim_msg = HwsimMsg {
        attributes: new_attr.attributes,
        hwsim_hdr: HwsimMsgHdr {
            hwsim_cmd: HwsimCmd::TxInfoFrame,
            hwsim_version: 0,
            reserved: hwsim_hdr.reserved,
        },
        nl_hdr: NlMsgHdr {
            nlmsg_len,
            nlmsg_type: NLMSG_MIN_TYPE,
            nlmsg_flags: nl_hdr.nlmsg_flags,
            nlmsg_seq: 0,
            nlmsg_pid: 0,
        },
    };
    Ok(new_hwsim_msg)
}
