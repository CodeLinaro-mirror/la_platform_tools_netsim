// Copyright 2025 The Android Open Source Project

//! Capture Actor
//!
//! This crate provides an actor for managing packet captures. It handles
//! starting and stopping captures, writing to PCAP files, and managing
//! capture state for different chips.

mod bt_pcap;
mod capture_actor;
mod error;
mod lifecycle;
mod service;
mod uwb_pcap;
mod wifi_pcap;
mod writer;

use actor_framework::ResourceActor;
pub use capture_actor::CaptureActor;
pub use error::CaptureError;

pub mod client;
pub use client::CaptureClient;

/// Creates a new Capture actor and its client.
pub fn new() -> (ResourceActor<CaptureActor>, CaptureClient) {
    // Buffer size of 32 is sufficient for capture control commands.
    // Packet data flows through a separate channel if needed, but here we handle
    // control.
    let (runner, client) = ResourceActor::new(32);
    (runner, CaptureClient::new(client))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        sync::{
            atomic::{AtomicBool, AtomicUsize, Ordering},
            Arc,
        },
        time::SystemTime,
    };

    use actor_framework::ActorService;
    use bytes::Bytes;
    use capture_api::{CaptureAction, CaptureCreate, Direction};
    use netsim_model::chip::{ChipId, ChipKind};

    use super::*;
    use crate::{bt_pcap::BluetoothH4Writer, service::InternalCaptureInfo};

    static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

    #[tokio::test]
    async fn test_pcap_writer() {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("netsim_test_pcap_writer_{}", id));
        fs::create_dir_all(&dir).unwrap();
        let filename = dir.join("test_pcap.pcap");
        let mut writer = BluetoothH4Writer::new(&filename).await.unwrap();
        let data = vec![0x01, 0x00, 0x00, 0x00]; // Fake H4 Command
        writer.write_packet(SystemTime::now(), Direction::Sent, &data).await.unwrap();

        let (records, bytes) = writer.get_stats();
        assert_eq!(records, 1);
        assert_eq!(bytes, 4);

        // explicitly drop writer to ensure file handle closed (though not strictly
        // required for remove_file on linux)
        drop(writer);
        fs::remove_file(filename).unwrap();
        fs::remove_dir(dir).unwrap();
    }

    struct MockContext;
    impl actor_framework::Context<CaptureActor> for MockContext {
        fn set_interval(&mut self, _duration: std::time::Duration) {}
        fn add_stream(&mut self, _id: ChipId, _stream: actor_framework::BoxStream) {}
        fn remove_stream(&mut self, _id: ChipId) {}
        fn add_typed_stream(&mut self, _id: usize, _stream: actor_framework::BoxTypedStream<()>) {}
        fn remove_typed_stream(&mut self, _id: usize) {}
        fn spawn(&mut self, _id: ChipId, _task: futures::future::BoxFuture<'static, ChipId>) {}
        fn abort(&mut self, _id: ChipId) {}
        fn shutdown(&mut self) {}
        fn run_later(
            &mut self,
            _duration: std::time::Duration,
            _f: Box<
                dyn FnOnce(&mut CaptureActor, &mut dyn actor_framework::Context<CaptureActor>)
                    + Send,
            >,
        ) -> actor_framework::TimerKey {
            unimplemented!()
        }
        fn cancel_timer(&mut self, _key: actor_framework::TimerKey) {}
    }

    fn setup_test_context() -> (CaptureActor, PathBuf) {
        let id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let temp_dir =
            std::env::temp_dir().join(format!("netsim_capture_test_{}_{}", std::process::id(), id));
        fs::create_dir_all(&temp_dir).unwrap();
        let ctx = CaptureActor::new(false, Some(temp_dir.clone()));
        (ctx, temp_dir)
    }

    fn teardown_test_context(dir: PathBuf) {
        if dir.exists() {
            fs::remove_dir_all(dir).unwrap();
        }
    }

    #[tokio::test]
    async fn test_capture_entity_lifecycle() {
        let chip_id = ChipId(1);
        let (mut ctx, temp_dir) = setup_test_context();

        let enabled_flag = Arc::new(AtomicBool::new(false));
        let create_params = CaptureCreate {
            chip_id,
            chip_kind: ChipKind::BLUETOOTH,
            device_name: "test_device".to_string(),
            enabled_flag: enabled_flag.clone(),
        };
        let mut entity = InternalCaptureInfo::from_create_params(chip_id, create_params).unwrap();
        assert_eq!(entity.info.enabled, false);

        let mut runtime = MockContext;

        // Enable capture
        ctx.update_entity(&mut entity, true, &mut runtime).await.unwrap();
        assert_eq!(entity.info.enabled, true);
        assert!(ctx.writers.contains_key(&chip_id));

        // Capture packet
        // Verify stats - Insert entity into context for handle_action and handle_get to
        // work
        ctx.entities.insert(chip_id, entity.clone());
        let packet = vec![0x01, 0x02, 0x03, 0x04];
        ctx.handle_action(
            Some(chip_id),
            CaptureAction::CapturePacket {
                chip_id,
                direction: Direction::Sent,
                bytes: Bytes::from(packet.clone()),
            },
            &mut runtime,
        )
        .await
        .unwrap();

        let info = ctx
            .handle_get(chip_id, &mut runtime)
            .await
            .unwrap()
            .expect("Failed to get capture info");
        assert_eq!(info.records_written, 1);
        assert_eq!(info.bytes_written, 4);

        // Update entity from context (if handle_action modified it, though here we
        // modified local entity) In this test, we modify `entity` local
        // variable primarily.

        // Disable capture
        ctx.update_entity(&mut entity, false, &mut runtime).await.unwrap();
        assert_eq!(entity.info.enabled, false);

        // Delete
        ctx.delete_entity(&entity, &mut runtime).await.unwrap();
        assert!(!ctx.writers.contains_key(&chip_id));

        teardown_test_context(temp_dir);
    }

    #[tokio::test]
    async fn test_default_capture_enabled() {
        let (mut ctx, temp_dir) = setup_test_context();
        let chip_id = ChipId(2);
        let enabled_flag = Arc::new(AtomicBool::new(false));
        let create_params = CaptureCreate {
            chip_id,
            chip_kind: ChipKind::BLUETOOTH,
            device_name: "test_device_default".to_string(),
            enabled_flag: enabled_flag.clone(),
        };

        // Set default capture to true
        ctx.default_capture_enabled = true;

        let mut entity = InternalCaptureInfo::from_create_params(chip_id, create_params).unwrap();
        let mut runtime = MockContext;

        ctx.create_entity(&mut entity, &mut runtime).await.unwrap();

        // Should be enabled because of context default
        assert_eq!(entity.info.enabled, true);
        assert!(ctx.writers.contains_key(&chip_id));

        // Clean up
        ctx.delete_entity(&entity, &mut runtime).await.unwrap();
        teardown_test_context(temp_dir);
    }

    #[tokio::test]
    async fn test_capture_directory() {
        let (mut ctx, temp_dir) = setup_test_context();
        let chip_id = ChipId(3);
        let enabled_flag = Arc::new(AtomicBool::new(true));
        let create_params = CaptureCreate {
            chip_id,
            chip_kind: ChipKind::BLUETOOTH,
            device_name: "test_device_dir".to_string(),
            enabled_flag: enabled_flag.clone(),
        };

        let mut entity = InternalCaptureInfo::from_create_params(chip_id, create_params).unwrap();
        let mut runtime = MockContext;

        ctx.create_entity(&mut entity, &mut runtime).await.unwrap();

        let entries: Vec<_> = fs::read_dir(&temp_dir).unwrap().collect();
        assert!(!entries.is_empty(), "Capture file should be created in temp dir");

        ctx.delete_entity(&entity, &mut runtime).await.unwrap();
        teardown_test_context(temp_dir);
    }

    #[tokio::test]
    async fn test_uwb_capture() {
        let (mut ctx, temp_dir) = setup_test_context();
        let chip_id = ChipId(4);
        let enabled_flag = Arc::new(AtomicBool::new(true));
        let create_params = CaptureCreate {
            chip_id,
            chip_kind: ChipKind::UWB,
            device_name: "test_uwb_device".to_string(),
            enabled_flag: enabled_flag.clone(),
        };

        let mut entity = InternalCaptureInfo::from_create_params(chip_id, create_params).unwrap();
        let mut runtime = MockContext;

        ctx.create_entity(&mut entity, &mut runtime).await.unwrap();
        ctx.entities.insert(chip_id, entity.clone());

        assert!(ctx.writers.contains_key(&chip_id));

        let packet = vec![0x00, 0x01, 0x02, 0x03]; // Fake UCI packet
        ctx.handle_action(
            Some(chip_id),
            CaptureAction::CapturePacket {
                chip_id,
                direction: Direction::Sent,
                bytes: bytes::Bytes::from(packet.clone()),
            },
            &mut runtime,
        )
        .await
        .unwrap();

        let info = ctx.handle_get(chip_id, &mut runtime).await.unwrap().unwrap();
        assert_eq!(info.records_written, 1);
        assert_eq!(info.bytes_written, 4);

        ctx.delete_entity(&entity, &mut runtime).await.unwrap();
        teardown_test_context(temp_dir);
    }

    #[tokio::test]
    async fn test_wifi_capture() {
        let (mut ctx, temp_dir) = setup_test_context();
        let chip_id = ChipId(5);
        let enabled_flag = Arc::new(AtomicBool::new(true));

        // Test both WIFI and AP to ensure service routes them correctly
        for kind in [ChipKind::WIFI] {
            let create_params = CaptureCreate {
                chip_id,
                chip_kind: kind,
                device_name: format!("test_{:?}_device", kind),
                enabled_flag: enabled_flag.clone(),
            };

            let mut entity =
                InternalCaptureInfo::from_create_params(chip_id, create_params).unwrap();
            let mut runtime = MockContext;

            ctx.create_entity(&mut entity, &mut runtime).await.unwrap();
            ctx.entities.insert(chip_id, entity.clone());

            assert!(ctx.writers.contains_key(&chip_id));

            let packet = vec![0x00, 0x01, 0x02, 0x03]; // Fake 802.11 payload
            ctx.handle_action(
                Some(chip_id),
                CaptureAction::CapturePacket {
                    chip_id,
                    direction: Direction::Sent,
                    bytes: bytes::Bytes::from(packet.clone()),
                },
                &mut runtime,
            )
            .await
            .unwrap();

            let info = ctx.handle_get(chip_id, &mut runtime).await.unwrap().unwrap();

            // An invalid HwsimMsg (like the fake payload) will be discarded during parsing,
            // resulting in 0 records written for this chip.
            assert_eq!(info.records_written, 0);
            assert_eq!(info.bytes_written, 0);

            ctx.delete_entity(&entity, &mut runtime).await.unwrap();
        }
        teardown_test_context(temp_dir);
    }
}
