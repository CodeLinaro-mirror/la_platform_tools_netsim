// Copyright 2025-2026 The Android Open Source Project

use actor_framework::{ActorLifecycle, DynContext};
use netsim_model::ChipId;
use netsim_packets::ieee80211::Ieee80211;

use crate::ap_actor::{ApActor, WIFI_STREAM_ID};

impl ActorLifecycle for ApActor {
    async fn on_start(&mut self, _ctx: &mut DynContext<Self>) {
        log::info!("ApActor started");
    }

    async fn on_tick(&mut self, ctx: &mut DynContext<Self>) {
        if let Some(sink) = &self.sink {
            let interval = self.beacon_interval.unwrap_or(200);
            let now = std::time::Instant::now();
            for ap in self.aps.values_mut() {
                if !ap.enabled {
                    continue;
                }

                // Process asynchronous delayed queues initially
                while let Some((time, _)) = ap.delayed_frames.front() {
                    if now >= *time {
                        if let Some((_, frame)) = ap.delayed_frames.pop_front() {
                            if sink.send(frame).is_err() {
                                log::warn!("Sink closed while transmitting delayed frame");
                                ctx.shutdown();
                                self.sink = None;
                                return;
                            }
                        }
                    } else {
                        break;
                    }
                }

                // Beacon generation logic
                if let Ok(frames) = self.manager.generate_beacon(ap, interval) {
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
    // We expect stream messages (Mgmt frames or Data frames if bridged)
    async fn on_stream(
        &mut self,
        stream_id: crate::ap_actor::ApId,
        msg: bytes::Bytes,
        ctx: &mut DynContext<Self>,
    ) {
        if stream_id.0 != WIFI_STREAM_ID {
            log::warn!("Received message on unknown stream_id: {}", stream_id);
            return;
        }
        let source_id = ChipId(0); // Placeholder until we lookup by MAC

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
                let interval = self.beacon_interval.unwrap_or(200);
                for ap in self.aps.values_mut() {
                    if let Ok(frames) = self.manager.handle_frame(
                        ap,
                        &msg,
                        &self.shared_keys,
                        interval,
                        source_id,
                        ctx,
                    ) {
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
                let interval = self.beacon_interval.unwrap_or(200);
                for ap in self.aps.values_mut() {
                    if ap.config.bssid == dest {
                        if let Ok(frames) = self.manager.handle_frame(
                            ap,
                            &msg,
                            &self.shared_keys,
                            interval,
                            source_id,
                            ctx,
                        ) {
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

    async fn on_stream_closed(
        &mut self,
        stream_id: crate::ap_actor::ApId,
        ctx: &mut DynContext<Self>,
    ) {
        if stream_id.0 == WIFI_STREAM_ID {
            log::info!("WIFI_STREAM_ID closed, stopping ApActor");
            ctx.shutdown();
        }
    }
    async fn on_task_closed(&mut self, _id: crate::ap_actor::ApId, _ctx: &mut DynContext<Self>) {}
}
