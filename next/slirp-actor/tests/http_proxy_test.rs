// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! BDD Integration Tests for HTTP Proxy Wiring in SlirpActor &
//! SlirpBackend::Native.

use actor_framework::ActorService;
use slirp_actor::{SlirpActor, SlirpBackend};

use super::test_driver::{MockContext, SlirpTestDriver};

async fn run_http_proxy_wiring_test(backend: SlirpBackend) {
    // Given a SlirpActor with HTTP proxy configured and specified backend
    let proxy_url = "http://127.0.0.1:8080".to_string();
    let config = slirp_actor::SlirpConfig::default();
    let actor = SlirpActor::new_with_backend(config, Some(proxy_url), None, backend).await;
    let mut ctx = MockContext::default();

    // When the actor initializes its backend
    let status = actor.handle_get(0, &mut ctx).await;

    // Then the backend type matches and actor initializes cleanly
    assert!(status.is_ok());
    assert_eq!(actor.backend(), backend);

    // Given a SlirpTestDriver with Native backend
    let mut driver = SlirpTestDriver::new(backend, None).await;

    // When checking driver initialization status
    let initialized = driver.is_initialized().await;

    // Then the driver reports the backend is initialized
    assert!(initialized);
}

#[tokio::test]
async fn test_http_proxy_wiring() {
    for backend in [SlirpBackend::CFfi, SlirpBackend::Native] {
        run_http_proxy_wiring_test(backend).await;
    }
}
