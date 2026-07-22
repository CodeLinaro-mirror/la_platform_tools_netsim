// Copyright 2023-2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! DNS Proxy and DHCP DNS option generation.

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::{Duration, Instant},
};

use log::{debug, info};
use netsim_packets::DnsHeader;
use zerocopy::FromBytes;

use crate::Config;

/// Encodes a list of domain names into a sequence of labels.
/// Each label is prefixed by its length, and each domain is terminated by a
/// zero-length label.
pub fn encode_dns_search_list(dns_search: &[String]) -> Vec<u8> {
    let mut dns_search_bytes = Vec::new();
    for domain in dns_search {
        for label in domain.split('.') {
            dns_search_bytes.push(label.len() as u8);
            dns_search_bytes.extend_from_slice(label.as_bytes());
        }
        dns_search_bytes.push(0);
    }
    dns_search_bytes
}

/// Creates a DHCPv4 option for the DNS search list (Option 119).
pub fn create_dns_search_option(config: &Config) -> Option<Vec<u8>> {
    config.dns_search.as_ref().map(|dns_search| {
        let dns_search_bytes = encode_dns_search_list(dns_search);
        let mut option = Vec::new();
        option.extend_from_slice(&[119, dns_search_bytes.len() as u8]);
        option.extend_from_slice(&dns_search_bytes);
        option
    })
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct DnsCacheKey {
    name: String,
    qtype: u16,
}

#[derive(Debug)]
struct DnsCacheEntry {
    ip: IpAddr,
    ttl: u32,
    created_at: Instant,
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct DnsRecentQueryKey {
    name: String,
    qtype: u16,
    client_addr: SocketAddr,
}

#[derive(Debug)]
struct DnsRecentQueryValue {
    last_sent_time: Instant,
    server_idx: usize,
}

pub enum DnsQueryResult {
    Cached(Vec<u8>), // Synthesized reply payload
    Forward(IpAddr), // Forward to this upstream DNS server
    Ignore,
}

pub struct DnsProxy {
    cache: HashMap<DnsCacheKey, DnsCacheEntry>,
    recent_queries: HashMap<DnsRecentQueryKey, DnsRecentQueryValue>,
}

impl Default for DnsProxy {
    fn default() -> Self {
        Self::new()
    }
}

impl DnsProxy {
    pub fn new() -> Self {
        Self { cache: HashMap::new(), recent_queries: HashMap::new() }
    }

    pub fn handle_query(
        &mut self,
        query_payload: &[u8],
        client_addr: SocketAddr,
        now: Instant,
        dns_servers: &[IpAddr],
    ) -> DnsQueryResult {
        let is_ipv6 = client_addr.is_ipv6();

        // Helper to get the default/configured server for the correct family
        let get_upstream_server = |servers: &[IpAddr]| {
            let server = servers.iter().find(|ip| ip.is_ipv6() == is_ipv6).copied();
            match (is_ipv6, server) {
                (true, Some(IpAddr::V6(ipv6))) => IpAddr::V6(ipv6),
                (true, _) => IpAddr::V6("2001:4860:4860::8888".parse().unwrap()),
                (false, Some(IpAddr::V4(ipv4))) => IpAddr::V4(ipv4),
                (false, _) => IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
            }
        };

        // Filter servers to match the query family
        let family_servers: Vec<IpAddr> =
            dns_servers.iter().filter(|ip| ip.is_ipv6() == is_ipv6).copied().collect();

        // 1. Parse the question
        let Some((qname, qtype, _, flags)) = parse_dns_query(query_payload) else {
            // FALLBACK: If we cannot parse it, just forward to the default/configured
            // server!
            debug!("DNS Proxy: Failed to parse query, falling back to direct forwarding");
            return DnsQueryResult::Forward(get_upstream_server(dns_servers));
        };

        debug!("DNS Proxy: Query for name='{qname}', type={qtype} from {client_addr}");

        // 2. Check cache (only A and AAAA)
        if qtype == 1 || qtype == 28 {
            let key = DnsCacheKey { name: qname.clone(), qtype };
            if let Some(entry) = self.cache.get(&key) {
                let age = now.saturating_duration_since(entry.created_at);
                if age.as_secs() < entry.ttl as u64 {
                    let remaining_ttl = entry.ttl - age.as_secs() as u32;
                    debug!(
                        "DNS Proxy: Cache hit for '{}' -> {} (TTL={})",
                        qname, entry.ip, remaining_ttl
                    );
                    let reply =
                        synthesize_dns_reply(query_payload, flags, qtype, remaining_ttl, entry.ip);
                    return DnsQueryResult::Cached(reply);
                } else {
                    debug!("DNS Proxy: Cache expired for '{qname}'");
                    self.cache.remove(&key);
                }
            }
        }

        // 3. Failover logic (reactive)
        if family_servers.is_empty() {
            // No servers of the correct family configured, use hardcoded fallback
            return DnsQueryResult::Forward(get_upstream_server(dns_servers));
        }

        let key = DnsRecentQueryKey { name: qname.clone(), qtype, client_addr };

        let server_idx = if let Some(val) = self.recent_queries.get_mut(&key) {
            let age = now.saturating_duration_since(val.last_sent_time);
            if age < Duration::from_secs(2) {
                // Duplicate query within 2s -> RETRY!
                let next_idx = (val.server_idx + 1) % family_servers.len();
                info!(
                    "DNS Proxy: Detected retry for '{qname}' from {client_addr}. Failing over to server {next_idx}"
                );
                val.server_idx = next_idx;
                val.last_sent_time = now;
                next_idx
            } else {
                // Old query, reset to server 0
                val.server_idx = 0;
                val.last_sent_time = now;
                0
            }
        } else {
            // New query
            self.recent_queries
                .insert(key, DnsRecentQueryValue { last_sent_time: now, server_idx: 0 });
            0
        };

        DnsQueryResult::Forward(family_servers[server_idx])
    }

    pub fn handle_reply(&mut self, reply_payload: &[u8], now: Instant) {
        let Some((qname, qtype, ip, ttl)) = parse_dns_reply(reply_payload) else {
            return;
        };

        if qtype == 1 || qtype == 28 {
            debug!("DNS Proxy: Caching '{qname}' -> {ip} (TTL={ttl})");
            let key = DnsCacheKey { name: qname, qtype };
            self.cache.insert(key, DnsCacheEntry { ip, ttl, created_at: now });
        }
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.recent_queries.clear();
    }
}

fn parse_dns_name(payload: &[u8], mut offset: usize) -> Option<(String, usize)> {
    let mut name = String::new();
    loop {
        if offset >= payload.len() {
            return None;
        }
        let len = payload[offset] as usize;
        offset += 1;
        if len == 0 {
            break;
        }
        if offset + len > payload.len() {
            return None;
        }
        let label = std::str::from_utf8(&payload[offset..offset + len]).ok()?;
        if !name.is_empty() {
            name.push('.');
        }
        name.push_str(label);
        offset += len;
    }
    Some((name, offset))
}

fn parse_dns_query(payload: &[u8]) -> Option<(String, u16, u16, u16)> {
    if payload.len() < 12 {
        return None;
    }
    let header = DnsHeader::read_from_bytes(&payload[..12]).ok()?;
    let tx_id = header.transaction_id.get();
    let flags = header.flags.get();
    let num_questions = header.num_questions.get();

    if num_questions != 1 {
        return None;
    }

    let (qname, offset) = parse_dns_name(payload, 12)?;
    if offset + 4 > payload.len() {
        return None;
    }
    let qtype = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
    Some((qname, qtype, tx_id, flags))
}

fn parse_dns_name_or_pointer(payload: &[u8], offset: usize) -> Option<(String, usize)> {
    if offset >= payload.len() {
        return None;
    }
    if (payload[offset] & 0xC0) == 0xC0 {
        if offset + 2 > payload.len() {
            return None;
        }
        Some((String::new(), offset + 2))
    } else {
        parse_dns_name(payload, offset)
    }
}

fn parse_dns_reply(payload: &[u8]) -> Option<(String, u16, IpAddr, u32)> {
    if payload.len() < 12 {
        return None;
    }
    let header = DnsHeader::read_from_bytes(&payload[..12]).ok()?;
    let num_questions = header.num_questions.get() as usize;
    let num_answers = header.num_answers.get() as usize;

    let mut offset = 12;

    let mut first_question = None;
    for _ in 0..num_questions {
        let (name, next_offset) = parse_dns_name(payload, offset)?;
        if next_offset + 4 > payload.len() {
            return None;
        }
        let qtype = u16::from_be_bytes([payload[next_offset], payload[next_offset + 1]]);
        if first_question.is_none() {
            first_question = Some((name, qtype));
        }
        offset = next_offset + 4;
    }

    let (qname, qtype) = first_question?;

    for _ in 0..num_answers {
        let (_, next_offset) = parse_dns_name_or_pointer(payload, offset)?;
        if next_offset + 10 > payload.len() {
            return None;
        }
        let rtype = u16::from_be_bytes([payload[next_offset], payload[next_offset + 1]]);
        let rclass = u16::from_be_bytes([payload[next_offset + 2], payload[next_offset + 3]]);
        let ttl = u32::from_be_bytes([
            payload[next_offset + 4],
            payload[next_offset + 5],
            payload[next_offset + 6],
            payload[next_offset + 7],
        ]);
        let rd_len =
            u16::from_be_bytes([payload[next_offset + 8], payload[next_offset + 9]]) as usize;

        offset = next_offset + 10;
        if offset + rd_len > payload.len() {
            return None;
        }

        let rdata = &payload[offset..offset + rd_len];
        offset += rd_len;

        if rclass == 1 {
            if rtype == 1 && rd_len == 4 {
                let ip = IpAddr::V4(Ipv4Addr::new(rdata[0], rdata[1], rdata[2], rdata[3]));
                return Some((qname, qtype, ip, ttl));
            } else if rtype == 28 && rd_len == 16 {
                let mut ip_bytes = [0u8; 16];
                ip_bytes.copy_from_slice(rdata);
                let ip = IpAddr::V6(ip_bytes.into());
                return Some((qname, qtype, ip, ttl));
            }
        }
    }

    None
}

fn synthesize_dns_reply(
    query_payload: &[u8],
    query_flags: u16,
    qtype: u16,
    ttl: u32,
    ip: IpAddr,
) -> Vec<u8> {
    let mut reply = query_payload.to_vec();

    // flags: Response (0x8000) | copied Opcode/RD | Recursion Available (0x0080)
    let reply_flags = 0x8000 | (query_flags & 0x7F00) | 0x0080;
    reply[2..4].copy_from_slice(&reply_flags.to_be_bytes());
    reply[6..8].copy_from_slice(&1u16.to_be_bytes()); // num_answers = 1

    // Pointer to question name: 0xC00C
    reply.extend_from_slice(&[0xC0, 0x0C]);
    // Type (2 bytes)
    reply.extend_from_slice(&qtype.to_be_bytes());
    // Class (2 bytes): IN (1)
    reply.extend_from_slice(&1u16.to_be_bytes());
    // TTL (4 bytes)
    reply.extend_from_slice(&ttl.to_be_bytes());

    match ip {
        IpAddr::V4(ipv4) => {
            // RdLength (2 bytes): 4
            reply.extend_from_slice(&4u16.to_be_bytes());
            // RData (4 bytes)
            reply.extend_from_slice(&ipv4.octets());
        }
        IpAddr::V6(ipv6) => {
            // RdLength (2 bytes): 16
            reply.extend_from_slice(&16u16.to_be_bytes());
            // RData (16 bytes)
            reply.extend_from_slice(&ipv6.octets());
        }
    }

    reply
}
