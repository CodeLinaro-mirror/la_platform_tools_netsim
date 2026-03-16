// Copyright 2025-2026 The Android Open Source Project

use actor_framework::ResourceClient;
use futures::TryFutureExt;
use netsim_model::{
    chip::{
        Chip, ChipClient, ChipConfig, ChipCreate, ChipId, ChipKindParams, ChipUpdate, ChipVariant,
    },
    chip_error::ChipError,
    client_error::ClientError,
    device::{DeviceId, Position},
    stats::NetsimRadioStats,
};

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
            .err_into::<ClientError>()
            .await?;
        Ok(())
    }

    /// Creates a new Access Point with the given configuration.
    ///
    /// Generates a random ID for the AP.
    pub async fn create_ap(&self, id: u32, config: crate::ApConfig) -> Result<(), ClientError> {
        let params = ChipCreate {
            device_id: DeviceId(0),
            packet_stream: None,
            packet_sink: None,
            config: ChipConfig {
                name: config.ssid.clone(),
                manufacturer: "Netsim".into(),
                product_name: "AccessPoint".into(),
                chip_kind_params: ChipKindParams::Ap(config.into()),
            },
        };
        self.client.create_with_id(ChipId(id), params).err_into::<ClientError>().await.map(|_| ())
    }

    /// Destroys an Access Point by ID.
    pub async fn destroy_ap(&self, id: u32) -> Result<(), ClientError> {
        self.client.delete(ChipId(id)).err_into::<ClientError>().await
    }

    /// Retrieves the state of an Access Point by ID.
    pub async fn get_ap(&self, id: u32) -> Result<Option<crate::ApState>, ClientError> {
        match self.client.get(ChipId(id)).err_into::<ClientError>().await? {
            Some(chip) => {
                if let Some(ChipVariant::Ap(ap_chip)) = chip.variant {
                    let config: crate::ApConfig = ap_chip
                        .config
                        .try_into()
                        .map_err(|e: String| ClientError::Chip(ChipError::Internal(e.into())))?;
                    Ok(Some(crate::ApState::new(config)))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Lists all active Access Points.
    pub async fn list_aps(&self) -> Result<Vec<crate::ApState>, ClientError> {
        let chips = self.client.list().err_into::<ClientError>().await?;
        let mut aps = Vec::new();
        for chip in chips {
            if let Some(ChipVariant::Ap(ap_chip)) = chip.variant {
                if let Ok(config) = ap_chip.config.try_into() {
                    aps.push(crate::ApState::new(config));
                }
            }
        }
        Ok(aps)
    }

    /// Updates existing Access Point configuration.
    pub async fn update_ap(
        &self,
        id: u32,
        ssid: Option<String>,
        position: Option<Position>,
    ) -> Result<crate::ApState, ClientError> {
        use netsim_model::chip::{ApUpdate, ChipUpdate, ChipVariantUpdate};
        let patch = ChipUpdate {
            name: ssid.clone(),
            variant: Some(ChipVariantUpdate::Ap(ApUpdate {
                ssid,
                channel: None,
                force_disconnect: Vec::new(),
            })),
            position,
            ..Default::default()
        };
        let chip = self.client.update(ChipId(id), patch).err_into::<ClientError>().await?;
        if let Some(ChipVariant::Ap(ap_chip)) = chip.variant {
            let config: crate::ApConfig = ap_chip
                .config
                .try_into()
                .map_err(|e: String| ClientError::Chip(ChipError::Internal(e.into())))?;
            Ok(crate::ApState::new(config))
        } else {
            Err(ClientError::Chip(ChipError::Internal("Updated chip is not an AP".into())))
        }
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
        use netsim_model::chip::{ApUpdate, ChipId, ChipUpdate, ChipVariantUpdate};

        let force_disconnect_macs = force_disconnect.unwrap_or_default();

        let patch = ChipUpdate {
            name: ssid.clone(),
            variant: Some(ChipVariantUpdate::Ap(ApUpdate {
                ssid,
                channel,
                force_disconnect: force_disconnect_macs,
            })),
            enabled,
            ..Default::default()
        };
        let chip = self.client.update(ChipId(id), patch).err_into::<ClientError>().await?;
        if let Some(ChipVariant::Ap(ap_chip)) = chip.variant {
            let config: crate::ApConfig = ap_chip
                .config
                .try_into()
                .map_err(|e: String| ClientError::Chip(ChipError::Internal(e.into())))?;
            Ok(crate::ApState::new(config))
        } else {
            Err(ClientError::Chip(ChipError::Internal("Updated chip is not an AP".into())))
        }
    }
}

#[async_trait::async_trait]
impl ChipClient for ApClient {
    async fn create(&self, id: ChipId, params: ChipCreate) -> Result<(), ClientError> {
        self.client.create_with_id(id, params).err_into::<ClientError>().await.map(|_| ())
    }

    async fn read(&self, id: ChipId) -> Result<Chip, ClientError> {
        self.client
            .get(id)
            .err_into::<ClientError>()
            .await?
            .ok_or(ClientError::Chip(netsim_model::chip_error::ChipError::ChipNotFound(id)))
    }

    async fn update(&self, id: ChipId, patch: ChipUpdate) -> Result<Chip, ClientError> {
        self.client.update(id, patch).err_into::<ClientError>().await
    }

    async fn delete(&self, id: ChipId) -> Result<(), ClientError> {
        self.client.delete(id).err_into::<ClientError>().await
    }

    async fn read_statistics(&self) -> Result<Box<[NetsimRadioStats]>, ClientError> {
        Ok(Box::new([]))
    }

    async fn read_count_for_testing(&self) -> Result<usize, ClientError> {
        self.client.list().err_into::<ClientError>().await.map(|chips| chips.len())
    }

    async fn shutdown(&self) -> Result<(), ClientError> {
        self.client.shutdown().err_into::<ClientError>().await
    }

    async fn reset(&self, _id: ChipId) -> Result<(), ClientError> {
        Ok(())
    }

    fn clone_box(&self) -> Box<dyn ChipClient> {
        Box::new(self.clone())
    }
}
