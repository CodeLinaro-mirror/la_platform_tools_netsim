// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! HTTP REST request dispatcher and endpoint handlers for netsim.

use std::{convert::Infallible, io::ErrorKind, sync::Arc};

use ap_actor::ApClient;
use device_actor::DeviceClient;
use http_body_util::{BodyExt, Full, LengthLimitError, Limited};
use hyper::{
    Method, Request, Response, StatusCode,
    body::{Body, Bytes, Incoming},
    header::{CONTENT_TYPE, HeaderValue, X_CONTENT_TYPE_OPTIONS},
    server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::TokioIo;
use link_api::LinkClient;
use tokio::{net::TcpListener, task::JoinSet};
use tracing::{debug, info, warn};

/// Largest request body accepted, in bytes. Bodies are buffered in memory
/// before dispatch, so this bounds what a single connection can allocate.
const MAX_BODY_BYTES: usize = 1 << 20;

/// Serves the `/v1` HTTP API on `listener` until the task is cancelled.
///
/// The caller binds the listener so it owns port selection and publishing the
/// bound port; this crate owns the accept loop. Only the REST endpoints are
/// served here today; the WebSocket endpoint at `/v1/websocket/bt` still has
/// its own listener.
pub async fn run(
    listener: TcpListener,
    device_client: DeviceClient,
    link_client: Arc<dyn LinkClient>,
    ap_client: ApClient,
    version: String,
) {
    match listener.local_addr() {
        Ok(addr) => info!("REST API server is listening on: {addr}"),
        Err(e) => warn!("REST API server failed to read local address: {e}"),
    }

    let server = Arc::new(RestServer::new(device_client, link_client, ap_client, version));
    let mut connection_tasks = JoinSet::new();

    loop {
        tokio::select! {
            accept_res = listener.accept() => {
                match accept_res {
                    Ok((socket, _)) => {
                        let server = Arc::clone(&server);
                        connection_tasks.spawn(async move {
                            let service = service_fn(move |req| {
                                let server = Arc::clone(&server);
                                async move { server.serve(req).await }
                            });
                            if let Err(e) =
                                http1::Builder::new().serve_connection(TokioIo::new(socket), service).await
                            {
                                debug!("REST connection error: {e}");
                            }
                        });
                    }
                    Err(e)
                        if matches!(
                            e.kind(),
                            ErrorKind::ConnectionReset
                                | ErrorKind::ConnectionAborted
                                | ErrorKind::Interrupted
                        ) =>
                    {
                        debug!("Transient error accepting connection on HTTP listener: {e}");
                    }
                    Err(e) => {
                        warn!("Error accepting connection on HTTP listener: {e}");
                        // Back off so a persistent accept error, such as running out
                        // of file descriptors, does not spin the task at full speed.
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                }
            }
            Some(res) = connection_tasks.join_next(), if !connection_tasks.is_empty() => {
                if let Err(e) = res {
                    debug!("REST connection task failed: {e}");
                }
            }
        }
    }
}

/// HTTP REST API Server context holding handles to core actor clients.
pub struct RestServer {
    pub device_client: DeviceClient,
    pub link_client: Arc<dyn LinkClient>,
    pub ap_client: ApClient,
    pub version: String,
}

/// Hand-written because the actor client handles are not `Debug`; only the
/// version is useful in diagnostics.
impl std::fmt::Debug for RestServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RestServer").field("version", &self.version).finish_non_exhaustive()
    }
}

fn build_response<B: Into<Bytes>>(status: StatusCode, body: B) -> Response<Full<Bytes>> {
    let mut resp = Response::new(Full::new(body.into()));
    *resp.status_mut() = status;
    resp.headers_mut().insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    resp.headers_mut().insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    resp
}

fn error_response(status: StatusCode, message: &str) -> Response<Full<Bytes>> {
    build_response(status, serde_json::json!({ "error": message }).to_string())
}

/// Helper that serializes to JSON or returns 500 automatically
fn json_response<T: serde::Serialize>(status: StatusCode, val: &T) -> Response<Full<Bytes>> {
    match serde_json::to_string(val) {
        Ok(json) => build_response(status, json),
        Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

impl RestServer {
    pub fn new(
        device_client: DeviceClient,
        link_client: Arc<dyn LinkClient>,
        ap_client: ApClient,
        version: String,
    ) -> Self {
        Self { device_client, link_client, ap_client, version }
    }

    /// Serves one HTTP request. Reads and bounds the body, then dispatches.
    ///
    /// The error type is [`Infallible`] because every failure is reported as an
    /// HTTP status rather than by dropping the connection.
    pub async fn serve(&self, req: Request<Incoming>) -> Result<Response<Full<Bytes>>, Infallible> {
        // Split the head from the body so the path can be borrowed rather than
        // copied; consuming the body would otherwise invalidate any borrow of
        // `req`.
        let (parts, body) = req.into_parts();
        let path = parts.uri.path();
        let path = path.strip_suffix('/').filter(|p| !p.is_empty()).unwrap_or(path);

        // Reject an oversized body from the declared length before reading it,
        // so a large upload is not buffered just to be rejected.
        if body.size_hint().lower() > MAX_BODY_BYTES as u64 {
            return Ok(error_response(StatusCode::PAYLOAD_TOO_LARGE, "413 Payload Too Large"));
        }
        // `Limited` stops reading once the cap is hit, which also covers a
        // chunked body whose length is not declared up front and so slips past
        // the check above.
        let body = match Limited::new(body, MAX_BODY_BYTES).collect().await {
            Ok(collected) => collected.to_bytes(),
            Err(e) if e.is::<LengthLimitError>() => {
                return Ok(error_response(StatusCode::PAYLOAD_TOO_LARGE, "413 Payload Too Large"));
            }
            Err(e) => {
                return Ok(error_response(
                    StatusCode::BAD_REQUEST,
                    &format!("400 Bad Request: {e}"),
                ));
            }
        };

        Ok(self.handle_request(&parts.method, path, &body).await)
    }

    /// Dispatches an incoming HTTP REST request path and method to the matching
    /// handler.
    pub async fn handle_request(
        &self,
        method: &Method,
        path: &str,
        _body: &[u8],
    ) -> Response<Full<Bytes>> {
        match (method, path) {
            (&Method::GET, "/v1/version") => {
                json_response(StatusCode::OK, &serde_json::json!({ "version": self.version }))
            }
            (&Method::GET, "/v1/devices") => match self.device_client.list().await {
                Ok(resp) => json_response(StatusCode::OK, &resp),
                Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
            },
            (&Method::GET, "/v1/links") => match self.link_client.list().await {
                Ok(links) => json_response(StatusCode::OK, &crate::ListLinkResponse { links }),
                Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
            },
            (&Method::GET, "/v1/aps") => match self.ap_client.list_aps().await {
                Ok(aps) => {
                    let aps = aps
                        .into_iter()
                        .map(|(_, state)| crate::Ap {
                            config: state.config.into(),
                            associations: {
                                // `ApState::associations` is a `HashSet<MacAddr>`, so format each
                                // `MacAddr` and sort to keep the JSON array order stable across calls.
                                let mut associations: Vec<String> =
                                    state.associations.into_iter().map(|m| m.to_string()).collect();
                                associations.sort();
                                associations
                            },
                        })
                        .collect();
                    json_response(StatusCode::OK, &crate::ListApResponse { aps })
                }
                Err(e) => error_response(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
            },
            _ => error_response(StatusCode::NOT_FOUND, &format!("Not Found: {} {}", method, path)),
        }
    }
}
