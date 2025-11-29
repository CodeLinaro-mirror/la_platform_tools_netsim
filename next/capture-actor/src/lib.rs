//! # Capture Actor Crate
//!
//! This crate provides the `CaptureActor`, which manages packet capture for simulated chips.
//! It handles creating PCAP files, writing packets to them, and managing capture state.
//!
//! The actor uses the `actor-framework` to manage its lifecycle and state.
//! It relies on `CaptureWriter`s to handle the actual writing of packets to different formats (e.g., PCAP).

//! Capture Actor
//!
//! This crate provides an actor for managing packet captures. It handles
//! starting and stopping captures, writing to PCAP files, and managing
//! capture state for different chips.

pub mod actions;
pub mod actor_impl;
pub mod bt_pcap;
pub mod context;
pub mod entity;
pub mod error;
pub mod handlers;
pub mod writer;

use crate::entity::CaptureEntity;
use actor_framework::{ResourceActor, ResourceClient};

/// Creates a new Capture actor and its client.
pub fn new() -> (ResourceActor<CaptureEntity>, ResourceClient<CaptureEntity>) {
    // Buffer size of 32 is sufficient for capture control commands.
    // Packet data flows through a separate channel if needed, but here we handle control.
    ResourceActor::new(32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::{CaptureAction, CaptureActionResult};
    use crate::bt_pcap::BluetoothH4Writer;
    use crate::context::CaptureContext;
    use crate::entity::CaptureEntity;
    use crate::writer::CaptureWriter;
    use actor_framework::ActorEntity;
    use bytes::Bytes;
    use capture_api::Direction;
    use netsim_model::chip::{ChipId, ChipKind};
    use std::collections::HashMap;
    use std::fs;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::SystemTime;

    #[test]
    fn test_pcap_writer() {
        let filename = "test_pcap.pcap";
        let mut writer = BluetoothH4Writer::new(filename).unwrap();
        let data = vec![0x01, 0x00, 0x00, 0x00]; // Fake H4 Command
        writer.write_packet(SystemTime::now(), Direction::Sent, &data).unwrap();

        let (records, bytes) = writer.get_stats();
        assert_eq!(records, 1);
        assert_eq!(bytes, 4);

        fs::remove_file(filename).unwrap();
    }

    #[tokio::test]
    async fn test_capture_entity_lifecycle() {
        let chip_id = ChipId(1);
        let ctx = CaptureContext::default();

        let enabled_flag = Arc::new(AtomicBool::new(false));
        let create_params = crate::actions::CaptureCreate {
            chip_id,
            chip_kind: ChipKind::BLUETOOTH,
            device_name: "test_device".to_string(),
            enabled_flag: enabled_flag.clone(),
        };
        let mut entity = CaptureEntity::from_create_params(chip_id, create_params).unwrap();
        assert_eq!(entity.enabled, false);
        assert_eq!(enabled_flag.load(Ordering::SeqCst), false);

        // Enable capture
        entity.on_update(true, &ctx).await.unwrap();
        assert_eq!(entity.enabled, true);
        assert_eq!(enabled_flag.load(Ordering::SeqCst), true);

        // Verify writer created
        {
            let writers = ctx.writers.lock().unwrap();
            assert!(writers.contains_key(&chip_id));
        }

        // Capture packet
        let packet = vec![0x01, 0x02, 0x03, 0x04];
        entity
            .handle_action(
                CaptureAction::CapturePacket {
                    chip_id,
                    direction: Direction::Sent,
                    bytes: Bytes::from(packet.clone()),
                },
                &ctx,
            )
            .await
            .unwrap();

        // Verify stats
        let action_get = CaptureAction::Get { chip_id };
        if let CaptureActionResult::Get(Some(info)) =
            entity.handle_action(action_get, &ctx).await.unwrap()
        {
            assert_eq!(info.records_written, 1);
            assert_eq!(info.bytes_written, 4);
        } else {
            panic!("Failed to get capture info");
        }

        // Disable capture
        entity.on_update(false, &ctx).await.unwrap();
        assert_eq!(entity.enabled, false);
        // Writer stays in context but is not used, or we could remove it.
        // Current implementation keeps it but we don't write to it if disabled.

        // Delete
        entity.on_delete(&ctx).await.unwrap();
        {
            let writers = ctx.writers.lock().unwrap();
            assert!(!writers.contains_key(&chip_id));
        }

        // Clean up file
        let filename = format!("capture_test_device_{}.pcap", chip_id.0);
        if fs::metadata(&filename).is_ok() {
            fs::remove_file(&filename).unwrap();
        }
    }

    #[tokio::test]
    async fn test_default_capture_enabled() {
        let ctx = CaptureContext::default();
        let chip_id = ChipId(2);
        let enabled_flag = Arc::new(AtomicBool::new(false));
        let create_params = crate::actions::CaptureCreate {
            chip_id,
            chip_kind: ChipKind::BLUETOOTH,
            device_name: "test_device_default".to_string(),
            default_enabled: false, // Override should be false, but context will enable it
            enabled_flag: enabled_flag.clone(),
        };

        // Set default capture to true in context
        ctx.default_capture_enabled.store(true, Ordering::SeqCst);

        let mut entity = CaptureEntity::from_create_params(chip_id, create_params).unwrap();
        entity.on_create(&ctx).await.unwrap();

        // Should be enabled because of context default
        assert_eq!(entity.enabled, true);
        assert_eq!(enabled_flag.load(Ordering::SeqCst), true);
        {
            let writers = ctx.writers.lock().unwrap();
            assert!(writers.contains_key(&chip_id));
        }

        // Clean up
        entity.on_delete(&ctx).await.unwrap();
        let filename = format!("capture_test_device_default_{}.pcap", chip_id.0);
        if fs::metadata(&filename).is_ok() {
            fs::remove_file(&filename).unwrap();
        }
    }

    #[tokio::test]
    async fn test_capture_directory() {
        let ctx = CaptureContext::default();
        let chip_id = ChipId(3);
        let enabled_flag = Arc::new(AtomicBool::new(false));
        let create_params = crate::actions::CaptureCreate {
            chip_id,
            chip_kind: ChipKind::BLUETOOTH,
            device_name: "test_device_dir".to_string(),
            default_enabled: true,
            enabled_flag: enabled_flag.clone(),
        };

        // Create a temp directory
        let temp_dir = std::env::temp_dir().join("netsim_capture_test");
        fs::create_dir_all(&temp_dir).unwrap();
        *ctx.capture_dir.lock().unwrap() = Some(temp_dir.clone());

        let mut entity = CaptureEntity::from_create_params(chip_id, create_params).unwrap();
        entity.on_create(&ctx).await.unwrap();

        // Verify writer created in temp dir
        let expected_path = temp_dir.join(format!("capture_test_device_dir_{}.pcap", chip_id.0));
        assert!(expected_path.exists());

        // Clean up
        entity.on_delete(&ctx).await.unwrap();
        if expected_path.exists() {
            fs::remove_file(&expected_path).unwrap();
        }
        fs::remove_dir(&temp_dir).unwrap();
    }
}
