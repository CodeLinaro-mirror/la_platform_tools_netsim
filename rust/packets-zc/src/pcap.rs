// Copyright 2024 The Android Open Source Project
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A minimal pcap parser.
//!
//! Note that the support for pcap format is very limited.
//! It only supports ethernet packets.

use crate::pcapng::{EnhancedPacketBlock, SectionHeaderBlock};
use std::io::{self, Read};
use zerocopy::{byteorder::little_endian, FromBytes, IntoBytes};

pub const PCAP_MAGIC_NUMBER: u32 = 0xa1b2c3d4;
pub const PCAPNG_MAGIC_NUMBER: u32 = 0x1A2B3C4D;

pub enum PcapReader<R: Read> {
    Pcap(LegacyPcapReader<R>),
    Pcapng(PcapngReader<R>),
}

impl<R: Read + 'static> PcapReader<R> {
    pub fn new(mut reader: R) -> anyhow::Result<PcapReader<Box<dyn Read>>> {
        let mut magic_buf = [0; 4];
        reader.read_exact(&mut magic_buf)?;
        let magic_number = u32::from_le_bytes(magic_buf);
        let remaining_reader = io::Cursor::new(magic_buf).chain(reader);
        let boxed_reader: Box<dyn Read> = Box::new(remaining_reader);

        if magic_number == PCAP_MAGIC_NUMBER {
            Ok(PcapReader::Pcap(LegacyPcapReader::new(boxed_reader)?))
        } else if magic_number == 0x0A0D0D0A {
            Ok(PcapReader::Pcapng(PcapngReader::new(boxed_reader)?))
        } else {
            anyhow::bail!("Invalid pcap magic number: {}", magic_number);
        }
    }

    pub fn next_record(&mut self) -> anyhow::Result<Option<(PcapRecordHeader, Vec<u8>)>> {
        match self {
            PcapReader::Pcap(reader) => reader.next_record(),
            PcapReader::Pcapng(reader) => reader.next_record(),
        }
    }
}

#[repr(C)]
#[derive(IntoBytes, FromBytes, Copy, Clone, Debug)]
pub struct PcapHeader {
    pub magic_number: little_endian::U32,
    pub version_major: little_endian::U16,
    pub version_minor: little_endian::U16,
    pub thiszone: little_endian::I32,
    pub sigfigs: little_endian::U32,
    pub snaplen: little_endian::U32,
    pub network: little_endian::U32,
}

#[repr(C)]
#[derive(IntoBytes, FromBytes, Copy, Clone, Debug)]
pub struct PcapRecordHeader {
    pub ts_sec: little_endian::U32,
    pub ts_usec: little_endian::U32,
    pub incl_len: little_endian::U32,
    pub orig_len: little_endian::U32,
}

pub struct LegacyPcapReader<R: Read> {
    reader: R,
    pub header: PcapHeader,
}

impl<R: Read> LegacyPcapReader<R> {
    pub fn new(mut reader: R) -> anyhow::Result<Self> {
        let mut header_buf = [0; std::mem::size_of::<PcapHeader>()];
        reader.read_exact(&mut header_buf)?;
        let header = PcapHeader::read_from_bytes(&header_buf[..]).unwrap();
        // The magic number check is already done in PcapReader::new
        Ok(Self { reader, header })
    }

    pub fn next_record(&mut self) -> anyhow::Result<Option<(PcapRecordHeader, Vec<u8>)>> {
        let mut header_buf = [0; std::mem::size_of::<PcapRecordHeader>()];
        if self.reader.read_exact(&mut header_buf).is_err() {
            return Ok(None);
        }
        let header = PcapRecordHeader::read_from_bytes(&header_buf[..]).unwrap();
        let mut data = vec![0; header.incl_len.get() as usize];
        self.reader.read_exact(&mut data)?;
        Ok(Some((header, data)))
    }
}

pub struct PcapngReader<R: Read> {
    reader: R,
    swap_bytes: bool,
}

impl<R: Read> PcapngReader<R> {
    pub fn new(mut reader: R) -> anyhow::Result<Self> {
        let mut shb_buf = [0; std::mem::size_of::<SectionHeaderBlock>()];
        reader.read_exact(&mut shb_buf)?;
        let shb = SectionHeaderBlock::read_from_bytes(&shb_buf[..]).unwrap();
        let swap_bytes = shb.byte_order_magic != PCAPNG_MAGIC_NUMBER;
        let block_total_length =
            if swap_bytes { shb.block_total_length.swap_bytes() } else { shb.block_total_length };
        let options_len = block_total_length as usize - std::mem::size_of::<SectionHeaderBlock>();
        io::copy(&mut reader.by_ref().take(options_len as u64), &mut io::sink())?;
        Ok(Self { reader, swap_bytes })
    }

    pub fn next_record(&mut self) -> anyhow::Result<Option<(PcapRecordHeader, Vec<u8>)>> {
        loop {
            let mut epb_buf = [0; std::mem::size_of::<EnhancedPacketBlock>()];
            if self.reader.read_exact(&mut epb_buf).is_err() {
                return Ok(None);
            }
            let epb = EnhancedPacketBlock::read_from_bytes(&epb_buf[..]).unwrap();
            let block_type =
                if self.swap_bytes { epb.block_type.swap_bytes() } else { epb.block_type };

            if block_type == 0x00000006 {
                let block_total_length = if self.swap_bytes {
                    epb.block_total_length.swap_bytes()
                } else {
                    epb.block_total_length
                };
                let captured_len =
                    if self.swap_bytes { epb.captured_len.swap_bytes() } else { epb.captured_len };
                let packet_len =
                    if self.swap_bytes { epb.packet_len.swap_bytes() } else { epb.packet_len };
                let mut data = vec![0; captured_len as usize];
                self.reader.read_exact(&mut data)?;
                let remaining_len = block_total_length as usize
                    - std::mem::size_of::<EnhancedPacketBlock>()
                    - captured_len as usize;
                if remaining_len > 0 {
                    io::copy(
                        &mut self.reader.by_ref().take(remaining_len as u64),
                        &mut io::sink(),
                    )?;
                }
                let record_header = PcapRecordHeader {
                    ts_sec: little_endian::U32::new(if self.swap_bytes {
                        epb.timestamp_high.swap_bytes()
                    } else {
                        epb.timestamp_high
                    }),
                    ts_usec: little_endian::U32::new(if self.swap_bytes {
                        epb.timestamp_low.swap_bytes()
                    } else {
                        epb.timestamp_low
                    }),
                    incl_len: little_endian::U32::new(captured_len),
                    orig_len: little_endian::U32::new(packet_len),
                };
                return Ok(Some((record_header, data)));
            } else {
                let block_total_length = if self.swap_bytes {
                    epb.block_total_length.swap_bytes()
                } else {
                    epb.block_total_length
                };
                let remaining_len =
                    block_total_length as usize - std::mem::size_of::<EnhancedPacketBlock>();
                io::copy(&mut self.reader.by_ref().take(remaining_len as u64), &mut io::sink())?;
            }
        }
    }
}
