// Copyright 2024 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use capture::pcap;
use std::io::Cursor;
use std::net::{IpAddr, Ipv6Addr};
use std::str::FromStr;
use tokio::io::BufReader;

fn ipv6_from_str(addr: &str) -> Result<IpAddr, std::io::Error> {
    match Ipv6Addr::from_str(addr) {
        Ok(addr) => Ok(addr.into()),
        Err(err) => Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, err.to_string())),
    }
}

#[tokio::test]
async fn dns_manager() -> Result<(), std::io::Error> {
    const DATA: &[u8] = include_bytes!("../../capture/data/dns.cap");

    let mut reader = BufReader::new(Cursor::new(DATA));
    let header = pcap::read_file_header(&mut reader).await?;
    assert_eq!(header.linktype, pcap::LinkType::Ethernet.into());
    let mut dns_manager = http_proxy::DnsManager::new();
    loop {
        match pcap::read_record(&mut reader).await {
            Ok((_hdr, record)) => {
                dns_manager.add_from_ethernet_slice(&record);
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(e) => {
                println!("Error: {:?}", e);
                assert!(false);
            }
        }
    }
    assert_eq!(dns_manager.len(), 4);

    //  0xf0d4 AAAA www.netbsd.org AAAA
    assert_eq!(
        dns_manager.get(&ipv6_from_str("2001:4f8:4:7:2e0:81ff:fe52:9a6b")?),
        Some("www.netbsd.org".into())
    );

    Ok(())
}
