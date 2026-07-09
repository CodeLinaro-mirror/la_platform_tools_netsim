// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap, VecDeque},
    sync::{Arc, atomic::Ordering as AtomicOrdering},
    time::{Duration, Instant},
};

use bytes::Bytes;
use netsim_model::{ModemAction, RadioTechnology, RegistrationStatus};
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

use crate::{
    constants::CALL_RING_TIMEOUT,
    metrics::{Metrics, MetricsSnapshot},
    modem::{ModemEffect, ModemEvent, ModemImpl},
    time::{Clock, SystemClock},
    types::{AT_OK, CommandAction, HostEvent, ModemError, ModemId, ModemSink},
};

#[derive(Debug)]
pub enum NetworkEvent {
    Response { id: ModemId, packet: Vec<u8> },
    NewConnection { id: ModemId, destination: String },
    ModemHungUp { id: ModemId },
    SinkError { id: ModemId },
}

// Internal representation of a scheduled event
#[derive(Debug)]
pub struct ScheduledEvent {
    pub when: Instant,
    pub modem_id: ModemId,
    pub event: ModemEvent,
}

impl PartialEq for ScheduledEvent {
    fn eq(&self, other: &Self) -> bool {
        self.when == other.when
    }
}

impl Eq for ScheduledEvent {}

impl PartialOrd for ScheduledEvent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledEvent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse ordering to make BinaryHeap a min-heap
        other.when.cmp(&self.when)
    }
}

/// Manages multiple modems and simulates network interactions.
pub struct ModemNetworkSimulator {
    event_queue: BinaryHeap<Reverse<ScheduledEvent>>,
    modems: HashMap<ModemId, ModemImpl>,
    sinks: HashMap<ModemId, ModemSink>,
    host_event_tx: mpsc::UnboundedSender<HostEvent>,
    metrics: Arc<Metrics>,
    clock: Arc<dyn Clock>,
}

impl ModemNetworkSimulator {
    /// Creates a new `ModemNetworkSimulator` with a real system clock.
    pub fn new(host_event_tx: tokio::sync::mpsc::UnboundedSender<HostEvent>) -> Self {
        Self::new_with_clock(Arc::new(SystemClock), host_event_tx)
    }

    /// Creates a new `ModemNetworkSimulator` with a specific clock for testing.
    pub fn new_with_clock(
        clock: Arc<dyn Clock>,
        host_event_tx: tokio::sync::mpsc::UnboundedSender<HostEvent>,
    ) -> Self {
        Self {
            modems: HashMap::new(),
            sinks: HashMap::new(),
            host_event_tx,
            event_queue: BinaryHeap::new(),
            metrics: Arc::new(Metrics::default()),
            clock,
        }
    }

    pub fn on_timer(&mut self, _id: ModemId) -> Vec<NetworkEvent> {
        self.tick().0
    }

    pub fn dispatch(&mut self, command: ModemAction) -> Vec<NetworkEvent> {
        match command {
            ModemAction::ProcessAtCommand { id, command } => self.send_at_command(id.0, &command),
            ModemAction::SetSignalStrength { id, rssi, ber } => {
                self.set_signal_strength(id.0, rssi, ber)
            }
            ModemAction::SetVoiceRegistration { id, status } => {
                self.set_voice_registration(id.0, status)
            }
            ModemAction::SetDataRegistration { id, status } => {
                self.set_data_registration(id.0, status)
            }
            ModemAction::IncomingCall { target_id, number } => {
                self.initiate_external_incoming_call(target_id.0, &number)
            }
            ModemAction::RemoteAnswer { id } => self.initiate_external_answer(id.0),
            ModemAction::RemoteHold { id, on_hold } => self.set_call_hold(id.0, on_hold),
            ModemAction::RemoteHangup { id } => self.initiate_external_hangup(id.0),
            ModemAction::IncomingSms { id, sender, text } => {
                self.send_incoming_sms(id.0, &sender, &text)
            }
            ModemAction::IncomingPdu { id, pdu } => self.send_incoming_pdu(id.0, &pdu),
            ModemAction::UpdateNetworkTime { id, time } => self.update_network_time(id.0, &time),
            ModemAction::UpdatePhysicalChannelConfigs { id } => {
                self.update_physical_channel_configs(id.0)
            }
            ModemAction::SetSimStatus { id, present } => self.set_sim_status(id.0, present),
            ModemAction::SetNetworkTechnology { id, tech } => {
                self.set_network_technology(id.0, tech)
            }
        }
    }

    pub fn set_sim_status(&mut self, id: ModemId, present: bool) -> Vec<NetworkEvent> {
        self.apply_to_modem(id, |modem| modem.set_sim_status(present))
    }

    pub fn set_network_technology(
        &mut self,
        id: ModemId,
        tech: RadioTechnology,
    ) -> Vec<NetworkEvent> {
        self.apply_to_modem(id, |modem| modem.set_network_technology(tech))
    }

    /// Creates a new modem instance.
    pub fn new_modem(
        &mut self,
        id: ModemId,
        sink: ModemSink,
        sim_type: Option<i32>,
    ) -> Result<(), ModemError> {
        self.new_modem_with_profile(id, sink, None, sim_type)
    }

    /// Creates a new modem instance with a specific SIM profile.
    pub fn new_modem_with_profile(
        &mut self,
        id: ModemId,
        sink: ModemSink,
        profile: Option<crate::config::SimProfile>,
        sim_type: Option<i32>,
    ) -> Result<(), ModemError> {
        if self.modems.contains_key(&id) {
            return Err(ModemError::DuplicateModemId(id));
        }
        let mut modem = crate::modem::ModemImpl::new(id, profile.unwrap_or_default());
        if let Some(t) = sim_type {
            modem.set_sim_status(t > 0);
        }
        if modem.phone_number().is_empty() {
            let num_modems = self.modems.len();
            let default_num = match num_modems {
                0 => "15555211001",
                1 => "15555211002",
                _ => "",
            };
            if !default_num.is_empty() {
                modem.set_phone_number(default_num);
            }
        }
        self.modems.insert(id, modem);
        self.sinks.insert(id, sink);
        Ok(())
    }

    /// Removes a modem instance.
    pub fn remove_modem(&mut self, id: ModemId) {
        self.modems.remove(&id);
        self.sinks.remove(&id);
    }

    /// Sends an AT command to a modem instance.
    pub fn send_at_command(&mut self, id: ModemId, cmd: &[u8]) -> Vec<NetworkEvent> {
        self.metrics.at_commands_received.fetch_add(1, AtomicOrdering::Relaxed);
        debug!("Received AT command for {}: {:?}", id, std::str::from_utf8(cmd));
        self.tick();
        self.apply_to_modem(id, |modem| modem.receive_at_command(cmd))
    }

    fn process_effects(&mut self, effects: Vec<(ModemId, ModemEffect)>) -> Vec<NetworkEvent> {
        let mut network_events = Vec::new();
        let mut queue = VecDeque::from(effects);

        while let Some((id, effect)) = queue.pop_front() {
            match effect {
                ModemEffect::Schedule { delay, event } => {
                    let msg = HostEvent::TimerRequest { chip_id: id, duration: delay };
                    if let Err(e) = self.host_event_tx.send(msg) {
                        error!("Failed to send timer request: {}", e);
                    }
                    self.schedule_event(id, delay, event);
                }
                ModemEffect::Response(packet) => {
                    debug!("Sending response to {}: {:?}", id, std::str::from_utf8(&packet));
                    if let Some(sink) = self.sinks.get_mut(&id)
                        && let Err(e) = sink.send(Bytes::from(packet))
                    {
                        error!("Failed to send response to modem {}: {}", id, e);
                        let event = HostEvent::SinkError(id);
                        if let Err(e) = self.host_event_tx.send(event) {
                            error!("Failed to send client sink error: {}", e);
                        }
                        network_events.push(NetworkEvent::SinkError { id });
                    }
                }
                ModemEffect::Action(action) => {
                    let (new_effects, events) = self.handle_command_action(id, action);
                    queue.extend(new_effects);
                    network_events.extend(events);
                }
            }
        }
        network_events
    }

    fn apply_to_modem<F>(&mut self, id: ModemId, f: F) -> Vec<NetworkEvent>
    where
        F: FnOnce(&mut ModemImpl) -> Vec<ModemEffect>,
    {
        let effects = if let Some(modem) = self.modems.get_mut(&id) {
            f(modem).into_iter().map(|e| (id, e)).collect()
        } else {
            Vec::new()
        };
        self.process_effects(effects)
    }

    // Helper to find peer ID.
    fn find_peer_id<P>(&self, source_id: ModemId, predicate: P) -> Option<ModemId>
    where
        P: Fn(&ModemImpl) -> bool,
    {
        self.modems.values().find(|m| m.id != source_id && predicate(m)).map(|m| m.id)
    }

    pub(crate) fn handle_command_action(
        &mut self,
        id: ModemId,
        action: CommandAction,
    ) -> (Vec<(ModemId, ModemEffect)>, Vec<NetworkEvent>) {
        debug!("[Network] Handling action from {}: {:?}", id, action);
        let mut effects = Vec::new();
        let mut events = Vec::new();

        match action {
            CommandAction::InitiateCall(phone_number) => {
                self.metrics.calls_initiated.fetch_add(1, AtomicOrdering::Relaxed);
                effects.extend(self.initiate_call(id, &phone_number));
            }
            CommandAction::InitiateCallAndHold(phone_number) => {
                self.metrics.calls_initiated.fetch_add(1, AtomicOrdering::Relaxed);

                // Find peer to hold
                let peer_to_hold = self.find_peer_id(id, |m| m.call_service.is_active());

                if let Some(peer_id) = peer_to_hold
                    && let Some(peer) = self.modems.get_mut(&peer_id)
                {
                    peer.call_service.receive_hold();
                }

                effects.extend(self.initiate_call(id, &phone_number));
                effects.push((id, ModemEffect::Response(AT_OK.to_vec())));
            }
            CommandAction::SwapCalls(active_peer, held_peer) => {
                if let Some(modem) = self.modems.get_mut(&active_peer) {
                    modem.call_service.receive_hold();
                }
                if let Some(modem) = self.modems.get_mut(&held_peer) {
                    modem.call_service.receive_resume();
                }
            }
            CommandAction::InitiateRemoteCall(phone_number) => {
                events.push(NetworkEvent::NewConnection { id, destination: phone_number.clone() });
                effects.extend(self.initiate_call(id, &phone_number));
                effects.push((id, ModemEffect::Response(AT_OK.to_vec())));
            }
            CommandAction::AnswerCall(answered_modem_id) => {
                self.metrics.calls_answered.fetch_add(1, AtomicOrdering::Relaxed);

                let caller_id =
                    self.find_peer_id(answered_modem_id, |m| m.call_service.is_dialing());

                if let Some(cid) = caller_id {
                    // Caller connects
                    if let Some(caller) = self.modems.get_mut(&cid)
                        && let Some(response) = caller.call_service.connect(cid, answered_modem_id)
                    {
                        effects.push((cid, ModemEffect::Response(response)));
                    }
                    // Callee connects
                    if let Some(callee) = self.modems.get_mut(&answered_modem_id)
                        && let Some(response) = callee.call_service.connect(answered_modem_id, cid)
                    {
                        effects.push((answered_modem_id, ModemEffect::Response(response)));
                    }
                }
            }
            CommandAction::HangupCall(hung_up_modem_id) => {
                self.metrics.calls_hung_up.fetch_add(1, AtomicOrdering::Relaxed);
                let mut effects = Vec::new();

                let remaining_peer_ids: Vec<ModemId> = if let Some(initiator_modem) =
                    self.modems.get(&hung_up_modem_id)
                {
                    initiator_modem.call_service.calls.iter().filter_map(|c| c.peer_id).collect()
                } else {
                    Vec::new()
                };

                for (&mid, modem) in self.modems.iter_mut() {
                    if mid != hung_up_modem_id {
                        let has_call_to_initiator = modem
                            .call_service
                            .calls
                            .iter()
                            .any(|c| c.peer_id == Some(hung_up_modem_id));

                        let initiator_still_has_call = remaining_peer_ids.contains(&mid);

                        if has_call_to_initiator && !initiator_still_has_call {
                            modem.call_service.receive_hangup_from_peer_id(hung_up_modem_id);
                            effects
                                .push((mid, ModemEffect::Response(b"\r\nNO CARRIER\r\n".to_vec())));
                        }
                    }
                }
                let processed_events = self.process_effects(effects);
                events.extend(processed_events);
                events.push(NetworkEvent::ModemHungUp { id: hung_up_modem_id });
            }
            CommandAction::InitiateEmergencyCall => {} // No-op
            CommandAction::ReceiveSms { to, pdu } => {
                self.metrics.sms_sent.fetch_add(1, AtomicOrdering::Relaxed);
                let peer_id = if let Some(ref num) = to {
                    let sender_num = self.modems.get(&id).map(|m| m.phone_number());
                    if sender_num.as_deref().map(normalize_number) == Some(normalize_number(num)) {
                        Some(id)
                    } else if let Some(peer) = self.find_peer_id(id, |m| {
                        normalize_number(&m.phone_number()) == normalize_number(num)
                    }) {
                        Some(peer)
                    } else {
                        warn!("No peer found with number {}, dropping SMS", num);
                        None
                    }
                } else {
                    debug!("No destination number, falling back to loopback (self)");
                    Some(id)
                };

                if let Some(peer_id) = peer_id {
                    let tpdu_len = crate::pdu::calculate_tpdu_len(&pdu);

                    let mut response = b"+CMT: ,".to_vec();
                    response.extend_from_slice(tpdu_len.to_string().as_bytes());
                    response.extend_from_slice(b"\r\n");
                    response.extend_from_slice(&pdu);
                    response.extend_from_slice(b"\r\n");
                    effects.push((peer_id, ModemEffect::Response(response)));
                }
            }
            CommandAction::ReceiveTextSms { to, text } => {
                self.metrics.sms_sent.fetch_add(1, AtomicOrdering::Relaxed);
                let sender_num = self.modems.get(&id).map(|m| m.phone_number());
                let peer_id =
                    if sender_num.as_deref().map(normalize_number) == Some(normalize_number(&to)) {
                        Some(id)
                    } else {
                        self.find_peer_id(id, |m| {
                            normalize_number(&m.phone_number()) == normalize_number(&to)
                        })
                    };

                if let Some(pid) = peer_id {
                    let sender_num_str = sender_num.unwrap_or_default();
                    if let Some(peer_modem) = self.modems.get_mut(&pid) {
                        let peer_effects = peer_modem.trigger_incoming_sms(&sender_num_str, &text);
                        effects.extend(peer_effects.into_iter().map(|e| (pid, e)));
                    }
                }
            }
            CommandAction::None => {} // No-op
        }
        (effects, events)
    }

    pub fn set_signal_strength(&mut self, id: ModemId, rssi: u8, ber: u8) -> Vec<NetworkEvent> {
        self.apply_to_modem(id, |modem| {
            modem.set_signal_strength(rssi, ber);
            Vec::new()
        })
    }

    pub fn set_voice_registration(
        &mut self,
        id: ModemId,
        status: RegistrationStatus,
    ) -> Vec<NetworkEvent> {
        self.apply_to_modem(id, |modem| modem.set_voice_registration(status))
    }

    pub fn set_data_registration(
        &mut self,
        id: ModemId,
        status: RegistrationStatus,
    ) -> Vec<NetworkEvent> {
        self.apply_to_modem(id, |modem| modem.set_data_registration(status))
    }

    pub fn initiate_external_incoming_call(
        &mut self,
        target_id: ModemId,
        number: &str,
    ) -> Vec<NetworkEvent> {
        self.apply_to_modem(target_id, |modem| modem.trigger_incoming_call(number))
    }

    pub fn initiate_external_answer(&mut self, id: ModemId) -> Vec<NetworkEvent> {
        self.apply_to_modem(id, |modem| modem.trigger_remote_answer())
    }

    pub fn set_call_hold(&mut self, id: ModemId, on_hold: bool) -> Vec<NetworkEvent> {
        self.apply_to_modem(id, |modem| modem.trigger_remote_hold(on_hold))
    }

    pub fn initiate_external_hangup(&mut self, id: ModemId) -> Vec<NetworkEvent> {
        self.apply_to_modem(id, |modem| modem.trigger_remote_hangup())
    }

    pub fn send_incoming_sms(
        &mut self,
        id: ModemId,
        sender: &str,
        text: &str,
    ) -> Vec<NetworkEvent> {
        self.apply_to_modem(id, |modem| modem.trigger_incoming_sms(sender, text))
    }

    pub fn send_incoming_pdu(&mut self, id: ModemId, pdu: &str) -> Vec<NetworkEvent> {
        self.apply_to_modem(id, |modem| modem.trigger_incoming_pdu(pdu))
    }

    pub fn update_network_time(&mut self, id: ModemId, time: &str) -> Vec<NetworkEvent> {
        self.apply_to_modem(id, |modem| modem.trigger_network_time_update(time))
    }

    pub fn update_physical_channel_configs(&mut self, id: ModemId) -> Vec<NetworkEvent> {
        self.apply_to_modem(id, |modem| {
            modem.data_service.on_update_physical_channel_configs(modem)
        })
    }

    /// Initiates a call from one modem to another (Internal).
    fn initiate_call(
        &mut self,
        caller_id: ModemId,
        phone_number: &str,
    ) -> Vec<(ModemId, ModemEffect)> {
        debug!("[Network] initiate_call: caller_id={}, phone_number={}", caller_id, phone_number);
        if tracing::enabled!(tracing::Level::DEBUG) {
            for (id, modem) in &self.modems {
                debug!("[Network]   modem id={}, phone_number='{}'", id, modem.phone_number());
            }
        }
        // Find target
        let normalized_target = normalize_number(phone_number);
        let target_id = self
            .find_peer_id(caller_id, |m| normalize_number(&m.phone_number()) == normalized_target);

        if let Some(tid) = target_id
            && let Some(modem) = self.modems.get_mut(&tid)
        {
            // Send RING
            let mut effects = modem.receive_at_command(b"RING\r\n");

            // Also schedule RING timeout on TARGET
            effects.push(ModemEffect::Schedule {
                delay: CALL_RING_TIMEOUT,
                event: ModemEvent::CallRingTimeout { call_token: 1 },
            });
            return effects.into_iter().map(|e| (tid, e)).collect();
        }
        Vec::new()
    }

    /// Ticks the event loop.
    pub fn tick(&mut self) -> (Vec<NetworkEvent>, Option<Duration>) {
        let now = self.clock.now();
        let mut effects = Vec::new();

        while let Some(event) = self.event_queue.peek() {
            if event.0.when <= now {
                let event = self.event_queue.pop().unwrap().0;

                if let Some(modem) = self.modems.get_mut(&event.modem_id) {
                    let new_effects: Vec<_> = modem
                        .handle_event(event.event)
                        .into_iter()
                        .map(|e| (event.modem_id, e))
                        .collect();
                    effects.extend(new_effects);
                }
            } else {
                break;
            }
        }
        let events = self.process_effects(effects);

        let next_duration = self.event_queue.peek().map(|e| {
            let when = e.0.when;
            if when > now { when - now } else { Duration::ZERO }
        });

        (events, next_duration)
    }

    /// Returns a snapshot of the current metrics.
    pub fn get_metrics(&self) -> MetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Returns the number of modem instances.
    pub fn get_modem_count(&self) -> usize {
        self.modems.len()
    }

    /// Returns a list of modem IDs.
    pub fn get_modem_ids(&self) -> Vec<ModemId> {
        self.modems.keys().copied().collect()
    }

    /// Returns a modem instance.
    pub fn get_modem(&self, id: ModemId) -> Option<&ModemImpl> {
        self.modems.get(&id)
    }

    pub fn get_modem_mut(&mut self, id: ModemId) -> Option<&mut ModemImpl> {
        self.modems.get_mut(&id)
    }

    pub fn external_echo_for_debug(&self, text: String) {
        debug!("[DEBUG] {}", text);
    }

    pub fn schedule_event(&mut self, modem_id: ModemId, delay: Duration, event: ModemEvent) {
        let when = self.clock.now() + delay;
        self.event_queue.push(Reverse(ScheduledEvent { when, modem_id, event }));
    }
}

fn normalize_number(num: &str) -> &str {
    num.strip_prefix('+').unwrap_or(num)
}
