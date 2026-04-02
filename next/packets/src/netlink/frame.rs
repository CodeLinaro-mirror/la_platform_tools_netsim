// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

/// Netlink Message Header
#[derive(
    FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout, Debug, Copy, Clone, PartialEq, Eq,
)]
#[repr(C, packed)]
pub struct NlMsgHdr {
    /// Length of message including header
    pub nlmsg_len: u32,
    /// Message type identifier
    pub nlmsg_type: u16,
    /// Flags (NLM_F_)
    pub nlmsg_flags: u16,
    /// Sequence number
    pub nlmsg_seq: u32,
    /// Sending process port ID
    pub nlmsg_pid: u32,
}

impl NlMsgHdr {
    pub const SIZE: usize = std::mem::size_of::<Self>();
}

/// Netlink Attribute Header
#[derive(
    FromBytes, IntoBytes, Unaligned, Immutable, KnownLayout, Debug, Copy, Clone, PartialEq, Eq,
)]
#[repr(C, packed)]
pub struct NlAttrHdr {
    /// Length of the attribute including header
    pub nla_len: u16,
    /// Attribute type and flags
    pub nla_type: u16,
}

impl NlAttrHdr {
    pub const NLA_F_NESTED: u16 = 1 << 15;
    pub const NLA_F_NET_BYTEORDER: u16 = 1 << 14;
    pub const NLA_TYPE_MASK: u16 = !(Self::NLA_F_NESTED | Self::NLA_F_NET_BYTEORDER);

    pub fn new(nla_len: u16, nla_type: u16, nested: bool, net_byteorder: bool) -> Self {
        let mut type_field = nla_type & Self::NLA_TYPE_MASK;
        if nested {
            type_field |= Self::NLA_F_NESTED;
        }
        if net_byteorder {
            type_field |= Self::NLA_F_NET_BYTEORDER;
        }
        Self { nla_len, nla_type: type_field }
    }

    pub fn type_(&self) -> u16 {
        self.nla_type & Self::NLA_TYPE_MASK
    }

    pub fn is_nested(&self) -> bool {
        (self.nla_type & Self::NLA_F_NESTED) != 0
    }

    pub fn is_net_byteorder(&self) -> bool {
        (self.nla_type & Self::NLA_F_NET_BYTEORDER) != 0
    }

    pub fn decode_full(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 4 {
            return Err("Packet too short for NlAttrHdr".into());
        }
        let (hdr, _) = zerocopy::Ref::<&[u8], NlAttrHdr>::from_prefix(bytes)
            .map_err(|_| "Failed to read NlAttrHdr")?;
        Ok(*hdr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nl_msg_hdr_layout() {
        assert_eq!(std::mem::size_of::<NlMsgHdr>(), 16);
    }

    #[test]
    fn test_nl_attr_hdr_layout() {
        assert_eq!(std::mem::size_of::<NlAttrHdr>(), 4);
    }

    #[test]
    fn test_nl_attr_hdr_flags() {
        let hdr = NlAttrHdr::new(4, 1, true, false);
        assert!(hdr.is_nested());
        assert!(!hdr.is_net_byteorder());
        assert_eq!(hdr.type_(), 1);
        let nla_type = hdr.nla_type;
        assert_eq!(nla_type, 1 | NlAttrHdr::NLA_F_NESTED);
    }
}
