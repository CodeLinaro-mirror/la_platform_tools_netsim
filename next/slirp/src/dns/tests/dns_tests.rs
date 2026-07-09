// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{
    net::{IpAddr, SocketAddr},
    time::{Duration, Instant},
};

use crate::dns::{DnsProxy, DnsQueryResult};

fn build_dns_query_bytes(tx_id: u16, name: &str, qtype: u16) -> Vec<u8> {
    let mut buf = vec![0u8; 512];
    let mut builder = netsim_packets::DnsPacketBuilder::new(&mut buf).unwrap();
    builder.transaction_id(tx_id);
    builder.flags(0x0100); // Recursion Desired
    builder.add_question(name, qtype, 1); // class IN
    let len = builder.build();
    buf.truncate(len);
    buf
}

fn build_dns_reply_bytes(tx_id: u16, name: &str, qtype: u16, ttl: u32, ip: IpAddr) -> Vec<u8> {
    let query = build_dns_query_bytes(tx_id, name, qtype);
    let mut reply = query.clone();
    let reply_flags: u16 = 0x8180; // Response, NoError
    reply[2..4].copy_from_slice(&reply_flags.to_be_bytes());
    reply[6..8].copy_from_slice(&1u16.to_be_bytes()); // answers = 1

    reply.extend_from_slice(&[0xC0, 0x0C]); // Pointer to name
    reply.extend_from_slice(&qtype.to_be_bytes());
    reply.extend_from_slice(&1u16.to_be_bytes()); // Class IN
    reply.extend_from_slice(&ttl.to_be_bytes());

    match ip {
        IpAddr::V4(ipv4) => {
            reply.extend_from_slice(&4u16.to_be_bytes()); // RdLength
            reply.extend_from_slice(&ipv4.octets());
        }
        IpAddr::V6(ipv6) => {
            reply.extend_from_slice(&16u16.to_be_bytes()); // RdLength
            reply.extend_from_slice(&ipv6.octets());
        }
    }
    reply
}

#[test]
fn test_dns_proxy_cache_hit_and_expiry() {
    let mut proxy = DnsProxy::new();
    let now = Instant::now();
    let client: SocketAddr = "10.0.2.15:12345".parse().unwrap();
    let servers = vec!["8.8.8.8".parse().unwrap()];

    // 1. Initial query -> Cache Miss, should Forward
    let query = build_dns_query_bytes(0x1234, "google.com", 1); // A query
    let res = proxy.handle_query(&query, client, now, &servers);
    assert!(matches!(res, DnsQueryResult::Forward(ip) if ip == servers[0]));

    // 2. Simulate upstream reply -> Cache it!
    let reply = build_dns_reply_bytes(0x1234, "google.com", 1, 300, "1.2.3.4".parse().unwrap());
    proxy.handle_reply(&reply, now);

    // 3. Query again -> Cache Hit!
    let res2 = proxy.handle_query(&query, client, now, &servers);
    let cached_payload = match res2 {
        DnsQueryResult::Cached(payload) => payload,
        _ => panic!("Expected Cache Hit"),
    };

    // Verify synthesized reply
    assert!(cached_payload.len() > query.len());
    let tx_id = u16::from_be_bytes([cached_payload[0], cached_payload[1]]);
    assert_eq!(tx_id, 0x1234);
    let flags = u16::from_be_bytes([cached_payload[2], cached_payload[3]]);
    assert_eq!(flags & 0x8000, 0x8000); // Response bit set
    let num_answers = u16::from_be_bytes([cached_payload[6], cached_payload[7]]);
    assert_eq!(num_answers, 1);

    // 4. Advance time by 299 seconds -> Still valid!
    let now_299 = now + Duration::from_secs(299);
    let res3 = proxy.handle_query(&query, client, now_299, &servers);
    assert!(matches!(res3, DnsQueryResult::Cached(_)));

    // 5. Advance time by 301 seconds -> Expired!
    let now_301 = now + Duration::from_secs(301);
    let res4 = proxy.handle_query(&query, client, now_301, &servers);
    assert!(matches!(res4, DnsQueryResult::Forward(_)));
}

#[test]
fn test_dns_proxy_failover() {
    let mut proxy = DnsProxy::new();
    let mut now = Instant::now();
    let client: SocketAddr = "10.0.2.15:12345".parse().unwrap();
    let servers = vec!["8.8.8.8".parse().unwrap(), "1.1.1.1".parse().unwrap()];

    let query = build_dns_query_bytes(0x1234, "google.com", 1);

    // 1. First query -> Forward to server 0
    let res1 = proxy.handle_query(&query, client, now, &servers);
    assert!(matches!(res1, DnsQueryResult::Forward(ip) if ip == servers[0]));

    // 2. Retry immediately (0.5s later) -> Failover to server 1!
    now += Duration::from_millis(500);
    let res2 = proxy.handle_query(&query, client, now, &servers);
    assert!(matches!(res2, DnsQueryResult::Forward(ip) if ip == servers[1]));

    // 3. Retry again (0.5s later) -> Cycle back to server 0!
    now += Duration::from_millis(500);
    let res3 = proxy.handle_query(&query, client, now, &servers);
    assert!(matches!(res3, DnsQueryResult::Forward(ip) if ip == servers[0]));

    // 4. New query (3s later, no retry) -> Reset to server 0!
    now += Duration::from_secs(3);
    let res4 = proxy.handle_query(&query, client, now, &servers);
    assert!(matches!(res4, DnsQueryResult::Forward(ip) if ip == servers[0]));
}
