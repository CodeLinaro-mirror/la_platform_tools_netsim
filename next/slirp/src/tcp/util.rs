// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use netsim_packets::TcpHeader;

pub(super) fn parse_tcp_options(
    tcp_header: &TcpHeader,
    packet: &[u8],
) -> (Option<u16>, Option<u8>, Vec<(u32, u32)>) {
    let mut mss = None;
    let mut window_scale = None;
    let mut sack_blocks = Vec::new();
    let data_offset = tcp_header.header_length();
    if data_offset <= std::mem::size_of::<TcpHeader>() {
        return (mss, window_scale, sack_blocks);
    }

    let mut options = &packet[std::mem::size_of::<TcpHeader>()..data_offset];
    while !options.is_empty() {
        let kind = options[0];
        match kind {
            0 => break,                   // End of options
            1 => options = &options[1..], // NOP
            2 => {
                // MSS
                if options.len() >= 4 && options[1] == 4 {
                    mss = Some(u16::from_be_bytes([options[2], options[3]]));
                    options = &options[4..];
                } else {
                    break;
                }
            }
            3 => {
                // Window Scale
                if options.len() >= 3 && options[1] == 3 {
                    window_scale = Some(options[2]);
                    options = &options[3..];
                } else {
                    break;
                }
            }
            5 => {
                // SACK
                if options.len() >= 2 {
                    let len = options[1] as usize;
                    if len >= 2 && (len - 2) % 8 == 0 {
                        let mut i = 2;
                        while i < len {
                            let left = u32::from_be_bytes([
                                options[i],
                                options[i + 1],
                                options[i + 2],
                                options[i + 3],
                            ]);
                            let right = u32::from_be_bytes([
                                options[i + 4],
                                options[i + 5],
                                options[i + 6],
                                options[i + 7],
                            ]);
                            sack_blocks.push((left, right));
                            i += 8;
                        }
                        options = &options[len..];
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            _ => {
                if options.len() >= 2 {
                    let len = options[1] as usize;
                    if len < 2 || len > options.len() {
                        break;
                    }
                    options = &options[len..];
                } else {
                    break;
                }
            }
        }
    }
    (mss, window_scale, sack_blocks)
}
