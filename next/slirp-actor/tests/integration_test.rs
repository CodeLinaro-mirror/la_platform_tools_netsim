// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

pub mod packet_helpers;
pub mod test_driver;

#[cfg(test)]
mod arp_icmp_test;
#[cfg(test)]
mod backend_perf_test;
#[cfg(test)]
mod backend_test;
#[cfg(test)]
mod deficiencies_test;
#[cfg(test)]
mod dhcp_dns_test;
#[cfg(test)]
mod http_proxy_test;
#[cfg(test)]
mod ip_frag_multicast_test;
#[cfg(test)]
mod ipv6_advanced_test;
#[cfg(test)]
mod mac_routing_test;
#[cfg(test)]
mod napt_test;
#[cfg(test)]
mod ndp_ipv6_test;
#[cfg(test)]
mod port_forwarding_test;
#[cfg(test)]
mod recent_fixes_test;
#[cfg(test)]
mod risk_areas_test;
#[cfg(test)]
mod snapshot_test;
#[cfg(test)]
mod tcp_options_test;
#[cfg(test)]
mod tcp_state_test;
#[cfg(test)]
mod tftp_test;

use actor_framework::{ActorLifecycle, ActorService, BoxStream, Context};
use futures::future::BoxFuture;
use slirp_actor::{SlirpActor, SlirpReq};
use tokio::sync::mpsc;
use zerocopy::FromBytes;

struct MockContext;

impl Context<SlirpActor> for MockContext {
    fn set_interval(&mut self, _duration: std::time::Duration) {}
    fn add_stream(&mut self, _id: u32, _stream: BoxStream) {}
    fn remove_stream(&mut self, _id: u32) {}
    fn add_typed_stream(
        &mut self,
        _id: usize,
        _stream: actor_framework::BoxTypedStream<bytes::Bytes>,
    ) {
    }
    fn remove_typed_stream(&mut self, _id: usize) {}
    fn spawn(&mut self, _id: u32, task: BoxFuture<'static, u32>) {
        tokio::spawn(task);
    }
    fn abort(&mut self, _id: u32) {}
    fn shutdown(&mut self) {}
    fn run_later(
        &mut self,
        _duration: std::time::Duration,
        _f: Box<dyn FnOnce(&mut SlirpActor, &mut dyn Context<SlirpActor>) + Send>,
    ) -> actor_framework::TimerKey {
        unimplemented!("run_later not implemented for MockContext")
    }
    fn cancel_timer(&mut self, _key: actor_framework::TimerKey) {}
}

#[tokio::test]
async fn test_slirp_actor_lifecycle() {
    // Given a SlirpActor
    let (tx_out, _rx_out) = mpsc::unbounded_channel::<bytes::Bytes>();
    let mut actor = SlirpActor::new(Default::default(), None, None).await;
    let mut ctx = MockContext;

    // Register Sink
    let (_stream_tx, stream_rx) = mpsc::unbounded_channel::<bytes::Bytes>();
    let stream = Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(stream_rx));
    let sink: netsim_model::PacketSink =
        Box::pin(futures::sink::unfold(tx_out, |tx, bytes| async move {
            let _ = tx.send(bytes);
            Ok(tx)
        }));
    let _ = actor
        .handle_action(
            None,
            SlirpReq::Register { client_id: 0, stream, sink, notifier: None },
            &mut ctx,
        )
        .await;

    // When I start the actor
    actor.on_start(&mut ctx).await;

    // Then it should initialize (we can check via handle_get)
    let status = actor.handle_get(0, &mut ctx).await;
    assert!(status.is_ok());
    assert!(status.unwrap().unwrap().initialized);

    // When I send a packet
    let dummy_packet = bytes::Bytes::from(vec![0u8; 64]);
    let req = SlirpReq::SendPacket(dummy_packet);

    // handle_action
    let result = actor.handle_action(None, req, &mut ctx).await;

    // Then the request should be handled successfully
    assert!(result.is_ok());

    // When I drop the actor, it shuts down (via Drop)
    drop(actor);
}

#[tokio::test]
async fn test_slirp_mac_learning_and_switching() {
    let (tx_out1, mut rx_out1) = mpsc::unbounded_channel::<bytes::Bytes>();
    let (tx_out2, mut rx_out2) = mpsc::unbounded_channel::<bytes::Bytes>();
    let mut actor = SlirpActor::new(Default::default(), None, None).await;
    let mut ctx = MockContext;

    let (_stream_tx1, stream_rx1) = mpsc::unbounded_channel::<bytes::Bytes>();
    let (_stream_tx2, stream_rx2) = mpsc::unbounded_channel::<bytes::Bytes>();
    let stream1: std::pin::Pin<Box<dyn tokio_stream::Stream<Item = bytes::Bytes> + Sync + Send>> =
        Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(stream_rx1));
    let stream2: std::pin::Pin<Box<dyn tokio_stream::Stream<Item = bytes::Bytes> + Sync + Send>> =
        Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(stream_rx2));

    let sink1: netsim_model::PacketSink =
        Box::pin(futures::sink::unfold(tx_out1, |tx, bytes| async move {
            let _ = tx.send(bytes);
            Ok(tx)
        }));
    let sink2: netsim_model::PacketSink =
        Box::pin(futures::sink::unfold(tx_out2, |tx, bytes| async move {
            let _ = tx.send(bytes);
            Ok(tx)
        }));
    let _ = actor
        .handle_action(
            None,
            SlirpReq::Register { client_id: 1, stream: stream1, sink: sink1, notifier: None },
            &mut ctx,
        )
        .await;
    let _ = actor
        .handle_action(
            None,
            SlirpReq::Register { client_id: 2, stream: stream2, sink: sink2, notifier: None },
            &mut ctx,
        )
        .await;

    actor.on_start(&mut ctx).await;

    // 1. Client 1 sends uplink packet with source MAC 00:11:22:33:44:55 (dest MAC
    //    00:00:00:00:00:00 is unknown unicast)
    let mut uplink_payload = vec![0u8; 64];
    uplink_payload[6..12].copy_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    actor.on_stream(1, bytes::Bytes::from(uplink_payload), &mut ctx).await;

    // Verify client 2 does not receive the uplink packet (uplink is unicast to
    // libslirp only)
    assert!(rx_out2.try_recv().is_err());

    // 2. Downlink unicast packet arrives for 00:11:22:33:44:55
    let mut downlink_unicast = vec![0u8; 64];
    downlink_unicast[0..6].copy_from_slice(&[0x00, 0x11, 0x22, 0x33, 0x44, 0x55]);
    actor.on_typed_stream(0, bytes::Bytes::from(downlink_unicast), &mut ctx).await;
    tokio::task::yield_now().await;

    // Verify client 1 receives it
    let pkt1 = rx_out1.try_recv();
    assert!(pkt1.is_ok());
    assert!(rx_out2.try_recv().is_err());

    // 3. Downlink broadcast packet arrives
    let mut downlink_bcast = vec![0u8; 64];
    downlink_bcast[0..6].copy_from_slice(&[0xff; 6]);
    actor.on_typed_stream(0, bytes::Bytes::from(downlink_bcast), &mut ctx).await;
    tokio::task::yield_now().await;

    // Verify both clients receive it
    assert!(rx_out1.try_recv().is_ok());
    assert!(rx_out2.try_recv().is_ok());
}

async fn run_slirp_backend_lifecycle_test(backend: slirp_actor::SlirpBackend) {
    let (tx_out, _rx_out) = mpsc::unbounded_channel::<bytes::Bytes>();
    let mut actor = SlirpActor::new_with_backend(Default::default(), None, None, backend).await;
    let mut ctx = MockContext;

    let (_stream_tx, stream_rx) = mpsc::unbounded_channel::<bytes::Bytes>();
    let stream = Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(stream_rx));
    let sink: netsim_model::PacketSink =
        Box::pin(futures::sink::unfold(tx_out, |tx, bytes| async move {
            let _ = tx.send(bytes);
            Ok(tx)
        }));
    let result = actor
        .handle_action(
            None,
            SlirpReq::Register { client_id: 0, stream, sink, notifier: None },
            &mut ctx,
        )
        .await;
    assert!(result.is_ok());

    actor.on_start(&mut ctx).await;

    let status = actor.handle_get(0, &mut ctx).await;
    assert!(status.is_ok());
    assert!(status.unwrap().unwrap().initialized);

    let dummy_packet = bytes::Bytes::from(vec![0u8; 64]);
    let req = SlirpReq::SendPacket(dummy_packet);
    let result = actor.handle_action(None, req, &mut ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_slirp_backend_lifecycle() {
    for backend in [slirp_actor::SlirpBackend::CFfi, slirp_actor::SlirpBackend::Native] {
        run_slirp_backend_lifecycle_test(backend).await;
    }
}

#[tokio::test]
async fn test_switch_backend() {
    let mut actor = SlirpActor::new(Default::default(), None, None).await;
    let mut ctx = MockContext;

    let result = actor
        .handle_action(None, SlirpReq::SwitchBackend(slirp_actor::SlirpBackend::Native), &mut ctx)
        .await;
    assert!(result.is_ok());
    assert_eq!(actor.backend(), slirp_actor::SlirpBackend::Native);
}

async fn run_slirp_backend_icmp_ping_test(backend: slirp_actor::SlirpBackend) {
    let (tx_out, _rx_out) = mpsc::unbounded_channel::<bytes::Bytes>();
    let mut actor = SlirpActor::new_with_backend(Default::default(), None, None, backend).await;
    let mut ctx = MockContext;

    let (_stream_tx, stream_rx) = mpsc::unbounded_channel::<bytes::Bytes>();
    let stream = Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(stream_rx));
    let sink: netsim_model::PacketSink =
        Box::pin(futures::sink::unfold(tx_out, |tx, bytes| async move {
            let _ = tx.send(bytes);
            Ok(tx)
        }));
    let result = actor
        .handle_action(
            None,
            SlirpReq::Register { client_id: 0, stream, sink, notifier: None },
            &mut ctx,
        )
        .await;
    assert!(result.is_ok());

    actor.on_start(&mut ctx).await;

    let icmp_header_len = std::mem::size_of::<netsim_packets::IcmpHeader>();
    let payload = b"netsim icmp ping payload";
    let total_icmp_len = icmp_header_len + payload.len();

    let mut icmp_data = vec![0u8; total_icmp_len];
    let (icmp_hdr_slice, echo_payload_slice) = icmp_data.split_at_mut(icmp_header_len);
    let icmp_header = netsim_packets::IcmpHeader::mut_from_bytes(icmp_hdr_slice).unwrap();
    icmp_header.icmp_type = 8;
    icmp_header.icmp_code = 0;
    icmp_header.icmp_checksum.set(0);
    icmp_header.rest[..2].copy_from_slice(&0x1234u16.to_be_bytes());
    icmp_header.rest[2..].copy_from_slice(&1u16.to_be_bytes());
    echo_payload_slice.copy_from_slice(payload);

    let checksum = netsim_packets::ipv4_checksum(&icmp_data);
    let icmp_header_mut =
        netsim_packets::IcmpHeader::mut_from_bytes(&mut icmp_data[..icmp_header_len]).unwrap();
    icmp_header_mut.icmp_checksum.set(checksum);

    let mut ip_data = vec![0u8; 20 + total_icmp_len];
    let mut ipv4_builder = netsim_packets::Ipv4Builder::new(
        &mut ip_data,
        netsim_packets::IP_P_ICMP,
        std::net::Ipv4Addr::new(10, 0, 2, 15),
        std::net::Ipv4Addr::new(8, 8, 8, 8),
    )
    .unwrap();
    ipv4_builder.payload(&icmp_data).unwrap();
    ipv4_builder.build().unwrap();

    let mut eth_data = vec![0u8; 14 + ip_data.len()];
    let (eth_header_slice, eth_payload_slice) = eth_data.split_at_mut(14);
    let eth_frame = netsim_packets::EthernetFrame::mut_from_bytes(eth_header_slice).unwrap();
    eth_frame.dst_addr = netsim_packets::MacAddr { bytes: [0x52, 0x54, 0x00, 0x12, 0x34, 0x56] };
    eth_frame.src_addr = netsim_packets::MacAddr { bytes: [0x02, 0x15, 0xb2, 0x00, 0x00, 0x00] };
    eth_frame.ethertype = 0x0800.into();
    eth_payload_slice.copy_from_slice(&ip_data);

    let icmp_packet = eth_data;

    let req = SlirpReq::SendPacket(bytes::Bytes::from(icmp_packet));
    let result = actor.handle_action(None, req, &mut ctx).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_slirp_backend_icmp_ping() {
    for backend in [slirp_actor::SlirpBackend::CFfi, slirp_actor::SlirpBackend::Native] {
        run_slirp_backend_icmp_ping_test(backend).await;
    }
}
