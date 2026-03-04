// Copyright 2025-2026 The Android Open Source Project

use actor_framework::ResourceClient;
use netsim_model::{chip_error::ChipError, client_error::ClientError, device::Position};

use super::{ApActor, ApReq};

/// Client for interacting with the Access Point Actor.
///
/// Wraps a `ResourceClient<ApActor>` and provides helper methods for
/// managing AP lifecycles and configuration.
#[derive(Clone)]
pub struct ApClient {
    pub client: ResourceClient<ApActor>,
    // Optional interceptor for capturing packets in tests
    #[cfg(feature = "testing")]
    interceptor: Option<
        std::sync::Arc<dyn Fn(&tokio::sync::mpsc::UnboundedSender<bytes::Bytes>) + Send + Sync>,
    >,
}

impl std::fmt::Debug for ApClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApClient").finish()
    }
}

impl ApClient {
    /// Create a new `ApClient` from a resource client.
    pub fn new(client: ResourceClient<ApActor>) -> Self {
        Self {
            client,
            #[cfg(feature = "testing")]
            interceptor: None,
        }
    }

    /// Create a new `ApClient` with a packet interceptor for testing.
    ///
    /// The interceptor callback is invoked when `register` is called,
    /// allowing tests to capture the output sink.
    #[cfg(feature = "testing")]
    pub fn new_with_interceptor(
        client: ResourceClient<ApActor>,
        interceptor: impl Fn(&tokio::sync::mpsc::UnboundedSender<bytes::Bytes>) + Send + Sync + 'static,
    ) -> Self {
        Self { client, interceptor: Some(std::sync::Arc::new(interceptor)) }
    }

    /// Registers the AP Actor with the packet stream and sink.
    ///
    /// This effectively starts the AP service loop for processing packets.
    pub async fn register(
        &self,
        stream: std::pin::Pin<Box<dyn tokio_stream::Stream<Item = bytes::Bytes> + Send>>,
        sink: tokio::sync::mpsc::UnboundedSender<bytes::Bytes>,
        shared_keys: std::sync::Arc<super::shared::SharedKeyStore>,
        beacon_interval: std::time::Duration,
    ) -> Result<(), ClientError> {
        #[cfg(feature = "testing")]
        if let Some(interceptor) = &self.interceptor {
            interceptor(&sink);
        }
        let _ = self
            .client
            .perform_action(None, ApReq::Register { stream, sink, shared_keys, beacon_interval })
            .await
            .map_err(|e| ClientError::Chip(ChipError::Internal(e.to_string())))?;
        Ok(())
    }

    /// Creates a new Access Point with the given configuration.
    ///
    /// Generates a random ID for the AP.
    pub async fn create_ap(&self, _id: u32, config: crate::ApConfig) -> Result<u32, ClientError> {
        let params: netsim_model::chip::ApCreate = config.into();
        self.client
            .create(params)
            .await
            .map(|id| id.0)
            .map_err(|e| ClientError::Send(e.to_string()))
    }

    /// Destroys an Access Point by ID.
    pub async fn destroy_ap(&self, id: u32) -> Result<(), ClientError> {
        self.client
            .delete(crate::ap_actor::ApId(id))
            .await
            .map_err(|e| ClientError::Send(e.to_string()))
    }

    /// Retrieves the state of an Access Point by ID.
    pub async fn get_ap(&self, id: u32) -> Result<Option<crate::ApState>, ClientError> {
        self.client
            .get(crate::ap_actor::ApId(id))
            .await
            .map_err(|e| ClientError::Send(e.to_string()))
    }

    /// Lists all active Access Points.
    pub async fn list_aps(&self) -> Result<Vec<crate::ApState>, ClientError> {
        self.client.list().await.map_err(|e| ClientError::Send(e.to_string()))
    }

    /// Updates existing Access Point configuration.
    pub async fn update_ap(
        &self,
        id: u32,
        ssid: Option<String>,
        position: Option<Position>,
    ) -> Result<crate::ApState, ClientError> {
        let patch = crate::ap_actor::ApActorUpdate {
            variant: crate::ap_actor::ApUpdate {
                ssid,
                channel: None,
                force_disconnect: Vec::new(),
            },
            position,
            enabled: None,
        };
        self.client
            .update(crate::ap_actor::ApId(id), patch)
            .await
            .map_err(|e| ClientError::Send(e.to_string()))
    }

    /// Updates Access Point configuration with advanced fields.
    pub async fn update_ap_config(
        &self,
        id: u32,
        ssid: Option<String>,
        channel: Option<u8>,
        force_disconnect: Option<Vec<String>>,
        enabled: Option<bool>,
    ) -> Result<crate::ApState, ClientError> {
        let force_disconnect_macs = force_disconnect.unwrap_or_default();

        let patch = crate::ap_actor::ApActorUpdate {
            variant: crate::ap_actor::ApUpdate {
                ssid,
                channel,
                force_disconnect: force_disconnect_macs,
            },
            position: None,
            enabled,
        };
        self.client
            .update(crate::ap_actor::ApId(id), patch)
            .await
            .map_err(|e| ClientError::Send(e.to_string()))
    }
    /// Disconnects a device from an Access Point.
    pub async fn disconnect(&self, id: u32, mac_str: String) -> Result<(), ClientError> {
        let mac = mac_str
            .parse::<netsim_packets::ethernet::MacAddr>()
            .map_err(|e| ClientError::Chip(ChipError::Internal(format!("Invalid MAC: {}", e))))?;
        self.client
            .perform_action(
                Some(crate::ap_actor::ApId(id)),
                ApReq::Disconnect { id: crate::ap_actor::ApId(id), mac },
            )
            .await
            .map(|_| ())
            .map_err(|e| ClientError::Chip(ChipError::Internal(e.to_string())))
    }
}
