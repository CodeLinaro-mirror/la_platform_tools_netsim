// Copyright 2025 The Android Open Source Project

//! Hwsim attributes parsing and building.

use crate::ethernet::MacAddr;
use crate::netlink::NlAttrHdr;
use crate::netlink::{mac80211_hwsim, HwsimAttrEnum, TxRate, TxRateFlag};
use std::fmt;
use zerocopy::IntoBytes;

/// Error type for Hwsim attribute parsing.
#[derive(Debug)]
pub enum HwsimError {
    /// Failed to parse a frame or attribute.
    Frame(String),
    /// Other errors.
    Other(String),
}

impl fmt::Display for HwsimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HwsimError::Frame(msg) => write!(f, "Frame error: {}", msg),
            HwsimError::Other(msg) => write!(f, "Other error: {}", msg),
        }
    }
}

impl std::error::Error for HwsimError {}

/// Result type for Hwsim operations.
pub type HwsimResult<T> = Result<T, HwsimError>;

/// Aligns a length to the specified alignment boundary (`NLA_ALIGNTO`).
fn nla_align(array_length: usize) -> usize {
    const NLA_ALIGNTO: usize = 4;
    array_length.wrapping_add(NLA_ALIGNTO - 1) & !(NLA_ALIGNTO - 1)
}

/// Builder for `HwsimAttrSet`.
#[derive(Default)]
pub struct HwsimAttrSetBuilder {
    transmitter: Option<MacAddr>,
    receiver: Option<MacAddr>,
    frame: Option<Vec<u8>>,
    flags: Option<u32>,
    rx_rate_idx: Option<u32>,
    signal: Option<u32>,
    cookie: Option<u64>,
    freq: Option<u32>,
    tx_info: Option<Vec<TxRate>>,
    tx_info_flags: Option<Vec<TxRateFlag>>,
    attributes: Vec<u8>,
}

/// Set of parsed Hwsim attributes.
#[derive(Debug)]
pub struct HwsimAttrSet {
    /// Transmitter MAC address.
    pub transmitter: Option<MacAddr>,
    /// Receiver MAC address.
    pub receiver: Option<MacAddr>,
    /// Raw frame data.
    pub frame: Option<Vec<u8>>,
    /// Flags.
    pub flags: Option<u32>,
    /// Receive rate index.
    pub rx_rate_idx: Option<u32>,
    /// Signal strength.
    pub signal: Option<u32>,
    /// Cookie.
    pub cookie: Option<u64>,
    /// Frequency.
    pub freq: Option<u32>,
    /// Transmit info (rates).
    pub tx_info: Option<Vec<TxRate>>,
    /// Transmit info flags.
    pub tx_info_flags: Option<Vec<TxRateFlag>>,
    /// Raw attributes bytes.
    pub attributes: Vec<u8>,
}

impl HwsimAttrSetBuilder {
    fn extend_attributes(&mut self, bytes: &[u8]) {
        let mut vec = bytes.to_vec();
        let nla_padding = nla_align(vec.len()) - vec.len();
        vec.extend(vec![0; nla_padding]);
        self.attributes.extend(vec);
    }

    /// Sets the transmitter address.
    pub fn transmitter(&mut self, transmitter: &[u8; 6]) -> &mut Self {
        let attr = mac80211_hwsim::HwsimAttrAddrTransmitter {
            header: NlAttrHdr::new(10, HwsimAttrEnum::AddrTransmitter as u16),
            address: *transmitter,
        };
        self.extend_attributes(attr.as_bytes());
        self.transmitter = Some(MacAddr::from(*transmitter));
        self
    }

    /// Sets the receiver address.
    pub fn receiver(&mut self, receiver: &[u8; 6]) -> &mut Self {
        let attr = mac80211_hwsim::HwsimAttrAddrReceiver {
            header: NlAttrHdr::new(10, HwsimAttrEnum::AddrReceiver as u16),
            address: *receiver,
        };
        self.extend_attributes(attr.as_bytes());
        self.receiver = Some(MacAddr::from(*receiver));
        self
    }

    /// Sets the frame data.
    pub fn frame(&mut self, frame: &[u8]) -> &mut Self {
        let len = (4 + frame.len()) as u16;
        let header = NlAttrHdr::new(len, HwsimAttrEnum::Frame as u16);
        self.attributes.extend_from_slice(header.as_bytes());
        self.attributes.extend_from_slice(frame);
        let nla_padding = nla_align(len as usize) - len as usize;
        self.attributes.extend(vec![0; nla_padding]);
        self.frame = Some(frame.to_vec());
        self
    }

    /// Sets the flags.
    pub fn flags(&mut self, flags: u32) -> &mut Self {
        let attr = mac80211_hwsim::HwsimAttrFlags {
            header: NlAttrHdr::new(8, HwsimAttrEnum::Flags as u16),
            flags,
        };
        self.extend_attributes(attr.as_bytes());
        self.flags = Some(flags);
        self
    }

    /// Sets the RX rate index.
    pub fn rx_rate(&mut self, rx_rate_idx: u32) -> &mut Self {
        let attr = mac80211_hwsim::HwsimAttrRxRate {
            header: NlAttrHdr::new(8, HwsimAttrEnum::RxRate as u16),
            rx_rate_idx,
        };
        self.extend_attributes(attr.as_bytes());
        self.rx_rate_idx = Some(rx_rate_idx);
        self
    }

    /// Sets the signal strength.
    pub fn signal(&mut self, signal: u32) -> &mut Self {
        let attr = mac80211_hwsim::HwsimAttrSignal {
            header: NlAttrHdr::new(8, HwsimAttrEnum::Signal as u16),
            signal,
        };
        self.extend_attributes(attr.as_bytes());
        self.signal = Some(signal);
        self
    }

    /// Sets the cookie.
    pub fn cookie(&mut self, cookie: u64) -> &mut Self {
        let attr = mac80211_hwsim::HwsimAttrCookie {
            header: NlAttrHdr::new(12, HwsimAttrEnum::Cookie as u16),
            cookie,
        };
        self.extend_attributes(attr.as_bytes());
        self.cookie = Some(cookie);
        self
    }

    /// Sets the frequency.
    pub fn freq(&mut self, freq: u32) -> &mut Self {
        let attr = mac80211_hwsim::HwsimAttrFreq {
            header: NlAttrHdr::new(8, HwsimAttrEnum::Freq as u16),
            freq,
        };
        self.extend_attributes(attr.as_bytes());
        self.freq = Some(freq);
        self
    }

    /// Sets the TX info.
    pub fn tx_info(&mut self, tx_info: &[TxRate]) -> &mut Self {
        let mut bytes = Vec::new();
        for rate in tx_info {
            bytes.extend_from_slice(rate.as_bytes());
        }
        let len = (4 + bytes.len()) as u16;
        let header = NlAttrHdr::new(len, HwsimAttrEnum::TxInfo as u16);
        self.attributes.extend_from_slice(header.as_bytes());
        self.attributes.extend_from_slice(&bytes);
        let nla_padding = nla_align(len as usize) - len as usize;
        self.attributes.extend(vec![0; nla_padding]);
        self.tx_info = Some(tx_info.to_vec());
        self
    }

    /// Sets the TX info flags.
    pub fn tx_info_flags(&mut self, tx_rate_flags: &[TxRateFlag]) -> &mut Self {
        let mut bytes = Vec::new();
        for flag in tx_rate_flags {
            bytes.extend_from_slice(flag.as_bytes());
        }
        let len = (4 + bytes.len()) as u16;
        let header = NlAttrHdr::new(len, HwsimAttrEnum::TxInfoFlags as u16);
        self.attributes.extend_from_slice(header.as_bytes());
        self.attributes.extend_from_slice(&bytes);
        let nla_padding = nla_align(len as usize) - len as usize;
        self.attributes.extend(vec![0; nla_padding]);
        self.tx_info_flags = Some(tx_rate_flags.to_vec());
        self
    }

    /// Builds the `HwsimAttrSet`.
    pub fn build(self) -> HwsimResult<HwsimAttrSet> {
        Ok(HwsimAttrSet {
            transmitter: self.transmitter,
            receiver: self.receiver,
            cookie: self.cookie,
            flags: self.flags,
            rx_rate_idx: self.rx_rate_idx,
            signal: self.signal,
            frame: self.frame,
            freq: self.freq,
            tx_info: self.tx_info,
            tx_info_flags: self.tx_info_flags,
            attributes: self.attributes,
        })
    }
}

impl fmt::Display for HwsimAttrSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ ")?;
        if let Some(v) = self.transmitter {
            write!(f, "transmitter: {v}, ")?
        }
        if let Some(v) = self.receiver {
            write!(f, "receiver: {v}, ")?
        }
        if let Some(v) = self.cookie {
            write!(f, "cookie: {v}, ")?
        }
        if let Some(v) = self.flags {
            write!(f, "flags: {v}, ")?
        }
        if let Some(v) = self.rx_rate_idx {
            write!(f, "rx_rate_idx: {v}, ")?
        }
        if let Some(v) = self.signal {
            write!(f, "signal: {v}, ")?
        }
        if let Some(v) = &self.frame {
            write!(f, "frame: {:?}, ", v)?
        }
        if let Some(v) = self.freq {
            write!(f, "freq: {v}, ")?
        }
        if let Some(v) = &self.tx_info {
            write!(f, "tx_info: {:?}, ", v)?
        }
        if let Some(v) = &self.tx_info_flags {
            write!(f, "tx_info_flags: {:?}, ", v)?
        }
        write!(f, "}}")
    }
}

impl HwsimAttrSet {
    /// Creates a new `HwsimAttrSetBuilder`.
    pub fn builder() -> HwsimAttrSetBuilder {
        HwsimAttrSetBuilder::default()
    }

    /// Parse and validates the attributes from a HwsimMsg command.
    pub fn parse(attributes: &[u8]) -> HwsimResult<HwsimAttrSet> {
        Self::parse_with_frame_transmitter(attributes, Option::None, Option::None)
    }

    /// Parse and validates the attributes from a HwsimMsg command.
    /// Update frame and transmitter if provided.
    pub fn parse_with_frame_transmitter(
        attributes: &[u8],
        frame: Option<&[u8]>,
        transmitter: Option<&[u8; 6]>,
    ) -> HwsimResult<HwsimAttrSet> {
        let mut index: usize = 0;
        let mut builder = HwsimAttrSet::builder();
        while index < attributes.len() {
            let nla_hdr = NlAttrHdr::decode_full(&attributes[index..index + 4])
                .map_err(|_| HwsimError::Frame("Failed to decode NlAttrHdr".into()))?;
            let nla_len = nla_hdr.nla_len.get() as usize;
            let attr_type = num_traits::FromPrimitive::from_u16(nla_hdr.type_());

            match attr_type {
                Some(HwsimAttrEnum::AddrTransmitter) => {
                    let attr = zerocopy::Ref::<&[u8], mac80211_hwsim::HwsimAttrAddrTransmitter>::from_prefix(&attributes[index..index+nla_len]).map_err(|_| HwsimError::Frame("Failed to read AddrTransmitter".into()))?.0;
                    builder.transmitter(&attr.address);
                }
                Some(HwsimAttrEnum::AddrReceiver) => {
                    let attr =
                        zerocopy::Ref::<&[u8], mac80211_hwsim::HwsimAttrAddrReceiver>::from_prefix(
                            &attributes[index..index + nla_len],
                        )
                        .map_err(|_| HwsimError::Frame("Failed to read AddrReceiver".into()))?
                        .0;
                    builder.receiver(&attr.address);
                }
                Some(HwsimAttrEnum::Frame) => {
                    if nla_len > 4 {
                        builder.frame(&attributes[index + 4..index + nla_len]);
                    }
                }
                Some(HwsimAttrEnum::Flags) => {
                    let attr = zerocopy::Ref::<&[u8], mac80211_hwsim::HwsimAttrFlags>::from_prefix(
                        &attributes[index..index + nla_len],
                    )
                    .map_err(|_| HwsimError::Frame("Failed to read Flags".into()))?
                    .0;
                    builder.flags(attr.flags);
                }
                Some(HwsimAttrEnum::RxRate) => {
                    let attr =
                        zerocopy::Ref::<&[u8], mac80211_hwsim::HwsimAttrRxRate>::from_prefix(
                            &attributes[index..index + nla_len],
                        )
                        .map_err(|_| HwsimError::Frame("Failed to read RxRate".into()))?
                        .0;
                    builder.rx_rate(attr.rx_rate_idx);
                }
                Some(HwsimAttrEnum::Signal) => {
                    let attr =
                        zerocopy::Ref::<&[u8], mac80211_hwsim::HwsimAttrSignal>::from_prefix(
                            &attributes[index..index + nla_len],
                        )
                        .map_err(|_| HwsimError::Frame("Failed to read Signal".into()))?
                        .0;
                    builder.signal(attr.signal);
                }
                Some(HwsimAttrEnum::Cookie) => {
                    let attr =
                        zerocopy::Ref::<&[u8], mac80211_hwsim::HwsimAttrCookie>::from_prefix(
                            &attributes[index..index + nla_len],
                        )
                        .map_err(|_| HwsimError::Frame("Failed to read Cookie".into()))?
                        .0;
                    builder.cookie(attr.cookie);
                }
                Some(HwsimAttrEnum::Freq) => {
                    let attr = zerocopy::Ref::<&[u8], mac80211_hwsim::HwsimAttrFreq>::from_prefix(
                        &attributes[index..index + nla_len],
                    )
                    .map_err(|_| HwsimError::Frame("Failed to read Freq".into()))?
                    .0;
                    builder.freq(attr.freq);
                }
                Some(HwsimAttrEnum::TxInfo) => {
                    let data = &attributes[index + 4..index + nla_len];
                    if data.len() % 2 == 0 {
                        let count = data.len() / 2;
                        let mut rates = Vec::with_capacity(count);
                        for i in 0..count {
                            let rate = zerocopy::Ref::<&[u8], TxRate>::from_prefix(&data[i * 2..])
                                .map_err(|_| HwsimError::Frame("Failed to read TxRate".into()))?
                                .0;
                            rates.push(*rate);
                        }
                        builder.tx_info(&rates);
                    }
                }
                _ => {}
            }
            index += nla_align(nla_len);
        }

        // Overrides
        if let Some(f) = frame {
            builder.frame(f);
        }
        if let Some(t) = transmitter {
            builder.transmitter(t);
        }

        builder.build()
    }
}
