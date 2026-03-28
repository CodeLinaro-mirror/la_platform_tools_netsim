// Copyright 2025 The Android Open Source Project

use actor_framework::{ActorLifecycle, ActorService, BoxStream, Context};
use futures::future::BoxFuture;
use slirp_actor::{SlirpActor, SlirpReq};
use tokio::sync::mpsc;

struct MockContext;

impl Context<SlirpActor> for MockContext {
    fn set_interval(&mut self, _duration: std::time::Duration) {}
    fn add_stream(&mut self, _id: u32, _stream: BoxStream) {}
    fn remove_stream(&mut self, _id: u32) {}
    fn add_typed_stream(&mut self, _id: usize, _stream: actor_framework::BoxTypedStream<()>) {}
    fn remove_typed_stream(&mut self, _id: usize) {}
    fn spawn(&mut self, _id: u32, _task: BoxFuture<'static, u32>) {}
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
    use tokio_stream::StreamExt;
    let stream = Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(stream_rx));
    let _ = actor.handle_action(None, SlirpReq::Register { stream, sink: tx_out }, &mut ctx).await;

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
