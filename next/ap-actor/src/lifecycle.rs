// Copyright 2025-2026 The Android Open Source Project

use crate::ap_actor::{ApActor, WIFI_STREAM_ID};
use crate::error::ApError;
use actor_framework::{ActorLifecycle, DynContext};
use async_trait::async_trait;
use netsim_packets::ieee80211::Ieee80211;

#[async_trait]
impl ActorLifecycle<u32> for ApActor {
    type Error = ApError;

    async fn on_start(&mut self, ctx: &mut DynContext<u32>) {
        log::info!("ApActor started");
    }

    async fn on_tick(&mut self, ctx: &mut DynContext<u32>) {
        // Beacon generation logic
        if let Some(sink) = &self.sink {
            for ap in self.aps.values() {
                if let Ok(frames) = self.manager.generate_beacon(ap) {
                    for frame in frames {
                        if sink.send(frame).is_err() {
                            log::warn!("Sink closed, stopping ApActor");
                            ctx.shutdown();
                            self.sink = None;
                            return;
                        }
                    }
                }
            }
        }
    }

    // We expect stream messages (Mgmt frames or Data frames if bridged)
    async fn on_stream(&mut self, stream_id: u32, msg: bytes::Bytes, ctx: &mut DynContext<u32>) {
        if stream_id != WIFI_STREAM_ID {
            log::warn!("Received message on unknown stream_id: {}", stream_id);
            return;
        }

        // Parse frame to get Destination Address (Addr1)
        let ieee_frame = match Ieee80211::decode(&msg) {
            Ok(frame) => frame,
            Err(_) => {
                log::warn!("Failed to parse IEEE 802.11 frame");
                return;
            }
        };

        let dest = ieee_frame.get_addr1();

        // If broadcast, send to all APs
        if dest.is_broadcast() {
            if let Some(sink) = &self.sink {
                for ap in self.aps.values_mut() {
                    if let Ok(frames) = self.manager.handle_frame(ap, &msg, &self.shared_keys, ctx)
                    {
                        for frame in frames {
                            let _ = sink.send(frame);
                        }
                    }
                }
            }
        } else {
            // Unicast - find matching AP by BSSID
            let mut handled = false;
            if let Some(sink) = &self.sink {
                for ap in self.aps.values_mut() {
                    if ap.config.bssid == dest {
                        if let Ok(frames) =
                            self.manager.handle_frame(ap, &msg, &self.shared_keys, ctx)
                        {
                            for frame in frames {
                                if sink.send(frame).is_err() {
                                    log::warn!("Sink closed, stopping ApActor");
                                    ctx.shutdown();
                                    self.sink = None;
                                    return;
                                }
                            }
                        }
                        handled = true;
                        break;
                    }
                }
            }

            if !handled {
                log::debug!("No AP found for unicast frame dest: {}", dest);
            }
        }
    }

    async fn on_stream_closed(&mut self, stream_id: u32, ctx: &mut DynContext<u32>) {
        if stream_id == WIFI_STREAM_ID {
            log::info!("WIFI_STREAM_ID closed, stopping ApActor");
            ctx.shutdown();
        }
    }
    async fn on_task_closed(&mut self, _id: u32, _ctx: &mut DynContext<u32>) {}
}
