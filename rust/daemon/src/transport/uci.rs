// Copyright 2023 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::empty_line_after_doc_comments)]

use bytes::Bytes;

use std::io::{Error, Read};

/// This module implements control packet parsing for UWB.
///
/// UWB Command Interface Specification, UCI Generic Specification
/// Version 1.1
///
/// 2.3.2 Format of Control Packets

const UCI_HEADER_SIZE: usize = 4;
const UCI_PAYLOAD_LENGTH_FIELD: usize = 3;

#[derive(Debug)]
pub struct Packet {
    pub payload: Bytes,
}

#[derive(Debug)]
pub enum PacketError {
    IoError(Error),
}

pub fn read_uci_packet<R: Read>(reader: &mut R) -> Result<Packet, PacketError> {
    // Read the UCI header
    let mut buffer = vec![0; UCI_HEADER_SIZE];
    reader.read_exact(&mut buffer[0..]).map_err(PacketError::IoError)?;
    // Extract the control packet payload length and read
    let length = buffer[UCI_PAYLOAD_LENGTH_FIELD] as usize + UCI_HEADER_SIZE;
    buffer.resize(length, 0);
    reader.read_exact(&mut buffer[UCI_HEADER_SIZE..]).map_err(PacketError::IoError)?;
    Ok(Packet { payload: Bytes::from(buffer) })
}
