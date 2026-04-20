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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nl_msg_hdr_layout() {
        assert_eq!(std::mem::size_of::<NlMsgHdr>(), 16);
    }
}
