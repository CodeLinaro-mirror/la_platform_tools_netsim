// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use actor_framework::{ActorLifecycle, ActorService, Context};
use bytes::Bytes;
use device_actor::DeviceActor;
use futures::future::BoxFuture;
use netsim_model::{ChipCreate, ChipId, DeviceId};
use nfc_actor::NfcActor;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

// --- Mock Helpers ---

#[derive(Clone)]
struct MockDeviceActorClient;

#[async_trait::async_trait]
impl actor_framework::ActorClient<DeviceActor> for MockDeviceActorClient {
    async fn create(
        &self,
        _params: <DeviceActor as ActorService>::Create,
    ) -> Result<
        <DeviceActor as ActorService>::Id,
        actor_framework::FrameworkError<<DeviceActor as ActorService>::Error>,
    > {
        panic!("not implemented");
    }
    async fn create_with_id(
        &self,
        _id: <DeviceActor as ActorService>::Id,
        _params: <DeviceActor as ActorService>::Create,
    ) -> Result<
        <DeviceActor as ActorService>::Id,
        actor_framework::FrameworkError<<DeviceActor as ActorService>::Error>,
    > {
        panic!("not implemented");
    }
    async fn get(
        &self,
        _id: <DeviceActor as ActorService>::Id,
    ) -> Result<
        Option<<DeviceActor as ActorService>::Entity>,
        actor_framework::FrameworkError<<DeviceActor as ActorService>::Error>,
    > {
        panic!("not implemented");
    }
    async fn update(
        &self,
        _id: <DeviceActor as ActorService>::Id,
        _update: <DeviceActor as ActorService>::Update,
    ) -> Result<
        <DeviceActor as ActorService>::Entity,
        actor_framework::FrameworkError<<DeviceActor as ActorService>::Error>,
    > {
        panic!("not implemented");
    }
    async fn delete(
        &self,
        _id: <DeviceActor as ActorService>::Id,
    ) -> Result<(), actor_framework::FrameworkError<<DeviceActor as ActorService>::Error>> {
        panic!("not implemented");
    }
    async fn perform_action(
        &self,
        _id: Option<<DeviceActor as ActorService>::Id>,
        _action: <DeviceActor as ActorService>::Action,
    ) -> Result<
        <DeviceActor as ActorService>::ActionResult,
        actor_framework::FrameworkError<<DeviceActor as ActorService>::Error>,
    > {
        Ok(device_api::DeviceActionResult::Success)
    }
    async fn list(
        &self,
    ) -> Result<
        Vec<<DeviceActor as ActorService>::Entity>,
        actor_framework::FrameworkError<<DeviceActor as ActorService>::Error>,
    > {
        panic!("not implemented");
    }
    async fn shutdown(
        &self,
    ) -> Result<(), actor_framework::FrameworkError<<DeviceActor as ActorService>::Error>> {
        panic!("not implemented");
    }
    fn clone_box(&self) -> Box<dyn actor_framework::ActorClient<DeviceActor>> {
        Box::new(self.clone())
    }
}

struct TestContext {
    tasks: Arc<Mutex<HashMap<ChipId, tokio::task::JoinHandle<ChipId>>>>,
    aborted_tasks: Arc<Mutex<Vec<ChipId>>>,
    removed_streams: Arc<Mutex<Vec<ChipId>>>,
}

impl Context<NfcActor> for TestContext {
    fn set_interval(&mut self, _duration: std::time::Duration) {}
    fn add_stream(&mut self, _id: ChipId, _stream: actor_framework::BoxStream) {}
    fn remove_stream(&mut self, id: ChipId) {
        self.removed_streams.lock().unwrap().push(id);
    }
    fn add_typed_stream(&mut self, _id: usize, _stream: actor_framework::BoxTypedStream<()>) {}
    fn remove_typed_stream(&mut self, _id: usize) {}
    fn spawn(&mut self, id: ChipId, task: BoxFuture<'static, ChipId>) {
        let handle = tokio::spawn(task);
        self.tasks.lock().unwrap().insert(id, handle);
    }
    fn abort(&mut self, id: ChipId) {
        self.aborted_tasks.lock().unwrap().push(id);
        let handle = self.tasks.lock().unwrap().remove(&id);
        if let Some(handle) = handle {
            handle.abort();
        }
    }
    fn shutdown(&mut self) {}
    fn run_later(
        &mut self,
        _duration: std::time::Duration,
        _f: actor_framework::TimerCallback<NfcActor>,
    ) -> actor_framework::TimerKey {
        panic!("not implemented");
    }
    fn cancel_timer(&mut self, _key: actor_framework::TimerKey) {}
}

struct MockSink(mpsc::Sender<Bytes>);
impl futures::Sink<Bytes> for MockSink {
    type Error = std::io::Error;
    fn poll_ready(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn start_send(self: std::pin::Pin<&mut Self>, item: Bytes) -> Result<(), Self::Error> {
        let _ = self.0.try_send(item);
        Ok(())
    }
    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }
}

// --- BDD World ---

struct NfcWorld {
    actor: NfcActor,
    ctx: TestContext,
    tasks: Arc<Mutex<HashMap<ChipId, tokio::task::JoinHandle<ChipId>>>>,
    aborted_tasks: Arc<Mutex<Vec<ChipId>>>,
    removed_streams: Arc<Mutex<Vec<ChipId>>>,
}

impl NfcWorld {
    async fn new() -> Self {
        let _ = tracing_subscriber::fmt().with_test_writer().try_init();

        let device_client = device_actor::DeviceClient::new(Box::new(MockDeviceActorClient));
        let mut actor = NfcActor::new(device_client);
        actor.start_casimir();

        let tasks = Arc::new(Mutex::new(HashMap::new()));
        let aborted_tasks = Arc::new(Mutex::new(Vec::new()));
        let removed_streams = Arc::new(Mutex::new(Vec::new()));
        let ctx = TestContext {
            tasks: tasks.clone(),
            aborted_tasks: aborted_tasks.clone(),
            removed_streams: removed_streams.clone(),
        };

        Self { actor, ctx, tasks, aborted_tasks, removed_streams }
    }

    async fn given_a_chip(&mut self, id_val: u32) {
        let chip_id = ChipId(id_val);
        let device_id = DeviceId(id_val);

        let (_tx_stream, rx_stream) = mpsc::channel(10);
        let (tx_sink, _rx_sink) = mpsc::channel(10);
        let packet_stream = ReceiverStream::new(rx_stream);
        let packet_sink = MockSink(tx_sink);

        let params = ChipCreate {
            packet_stream: Some(Box::new(packet_stream)),
            packet_sink: Some(Box::pin(packet_sink)),
            chip: netsim_model::Chip { device_id, ..Default::default() },
        };

        let create_res = self.actor.handle_create(Some(chip_id), params, &mut self.ctx).await;
        assert!(create_res.is_ok());
        assert_eq!(create_res.unwrap(), chip_id);
        assert!(self.actor.active_chips.contains_key(&chip_id));
        assert!(
            !self
                .actor
                .active_chips
                .get(&chip_id)
                .unwrap()
                .enabled
                .load(std::sync::atomic::Ordering::Relaxed),
            "Chip must be disabled by default upon boot!"
        );
    }

    async fn when_casimir_disconnects(&mut self, id_val: u32) {
        let chip_id = ChipId(id_val);
        let casimir_device_id = self.actor.active_chips.get(&chip_id).unwrap().casimir_device_id;

        let scene_client = self.actor.scene_client.as_ref().unwrap().clone();
        let remove_res = scene_client.remove_device(casimir_device_id).await;
        assert!(remove_res.is_ok());

        let handle = {
            let mut map = self.tasks.lock().unwrap();
            map.remove(&chip_id)
        };
        if let Some(handle) = handle {
            let res = handle.await;
            assert!(res.is_ok());
            assert_eq!(res.unwrap(), chip_id);

            self.actor.on_task_closed(chip_id, &mut self.ctx).await;
        } else {
            panic!("Task handle NOT found for chip {}", id_val);
        }
    }

    async fn then_chip_is_cleaned_up(&mut self, id_val: u32) {
        let chip_id = ChipId(id_val);
        assert!(!self.actor.active_chips.contains_key(&chip_id), "Chip still active in NfcActor");
        assert!(self.aborted_tasks.lock().unwrap().contains(&chip_id), "Task was not aborted");
        assert!(self.removed_streams.lock().unwrap().contains(&chip_id), "Stream was not removed");
    }
}

// --- Tests ---

// Feature: Casimir Disconnect Cleanup
// Scenario: Casimir disconnects and triggers clean EOF
//
//   Given a new NFC world with a registered chip
//   When Casimir disconnects the device
//   Then the chip is cleanly removed and tasks are aborted
#[tokio::test]
async fn test_casimir_eof_cleanup() {
    // Given
    let mut world = NfcWorld::new().await;
    let chip_id = 1;
    world.given_a_chip(chip_id).await;

    // When
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        world.when_casimir_disconnects(chip_id).await;
    })
    .await;
    assert!(timeout.is_ok(), "Test timed out! Possible hang detected.");

    // Then
    world.then_chip_is_cleaned_up(chip_id).await;
}

// Feature: NFC Enablement Gating
// Scenario: Chips spawn disabled by default and can be dynamically
// enabled/disabled
//
//   Given a new NFC world with a registered chip
//   Then the chip is disabled by default upon boot
//   When handle_update sets enabled = true
//   Then the chip state transitions to enabled
//   When handle_update sets enabled = false
//   Then the chip state transitions back to disabled
#[tokio::test]
async fn test_nfc_default_disabled_and_enablement_toggle() {
    // Given
    let mut world = NfcWorld::new().await;
    let chip_id = 1;
    world.given_a_chip(chip_id).await;

    // Then (Verify disabled by default upon boot)
    let state_disabled =
        world.actor.handle_get(ChipId(chip_id), &mut world.ctx).await.unwrap().unwrap();
    assert!(!state_disabled.enabled, "Chip should be disabled by default upon boot!");

    // When (Android OS calls NfcAdapter.enable())
    let update_enable = netsim_model::ChipUpdate { enabled: Some(true), ..Default::default() };
    let update_res1 =
        world.actor.handle_update(ChipId(chip_id), update_enable, &mut world.ctx).await;
    assert!(update_res1.is_ok());

    // Then (Verify chip is now enabled)
    let state_enabled =
        world.actor.handle_get(ChipId(chip_id), &mut world.ctx).await.unwrap().unwrap();
    assert!(state_enabled.enabled, "Chip should be enabled after update!");

    // When (Android OS calls NfcAdapter.disable())
    let update_disable = netsim_model::ChipUpdate { enabled: Some(false), ..Default::default() };
    let update_res2 =
        world.actor.handle_update(ChipId(chip_id), update_disable, &mut world.ctx).await;
    assert!(update_res2.is_ok());

    // Then (Verify chip is disabled again)
    let state_disabled_again =
        world.actor.handle_get(ChipId(chip_id), &mut world.ctx).await.unwrap().unwrap();
    assert!(!state_disabled_again.enabled, "Chip should be disabled after second update!");
}

// variant update path
//
// Given a new NFC world with a registered chip
// When handle_update sets state via variant = Some(true/false)
// Then the chip state transitions between enabled and disabled
#[tokio::test]
async fn test_nfc_update_via_variant() {
    // Given
    let mut world = NfcWorld::new().await;
    let chip_id = 1;
    world.given_a_chip(chip_id).await;

    // When (Update via variant field to enable)
    let update_enable = netsim_model::ChipUpdate {
        variant: Some(netsim_model::ChipVariantUpdate::Nfc(netsim_model::NfcUpdate {
            radio: netsim_model::RadioUpdate { state: Some(true) },
        })),
        ..Default::default()
    };
    let update_res1 =
        world.actor.handle_update(ChipId(chip_id), update_enable, &mut world.ctx).await;
    assert!(update_res1.is_ok());

    // Then (Verify chip is now enabled)
    let state_enabled =
        world.actor.handle_get(ChipId(chip_id), &mut world.ctx).await.unwrap().unwrap();
    assert!(state_enabled.enabled, "Chip should be enabled after variant update!");

    // When (Update via variant field to disable)
    let update_disable = netsim_model::ChipUpdate {
        variant: Some(netsim_model::ChipVariantUpdate::Nfc(netsim_model::NfcUpdate {
            radio: netsim_model::RadioUpdate { state: Some(false) },
        })),
        ..Default::default()
    };
    let update_res2 =
        world.actor.handle_update(ChipId(chip_id), update_disable, &mut world.ctx).await;
    assert!(update_res2.is_ok());

    // Then (Verify chip is disabled again)
    let state_disabled_again =
        world.actor.handle_get(ChipId(chip_id), &mut world.ctx).await.unwrap().unwrap();
    assert!(!state_disabled_again.enabled, "Chip should be disabled after second variant update!");
}
