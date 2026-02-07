use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap},
    sync::{atomic::Ordering as AtomicOrdering, Arc, Mutex},
    time::{Duration, Instant},
};

use log;
use netsim_model::chip::{Chip, ChipId};

use crate::time::{Clock, SystemClock}; // touch
use crate::{
    constants::CALL_RING_TIMEOUT,
    metrics::{Metrics, MetricsSnapshot},
    modem::{Modem, ModemEvent},
    modem_network::{ModemCallbacks, ModemError as NetworkError, ModemNetworkInterface},
    types::{Callbacks, CallbacksExt, CommandAction, ModemError, ModemId, NetworkCallbacks},
};

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
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScheduledEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering to make BinaryHeap a min-heap
        other.when.cmp(&self.when)
    }
}

/// The `ModemNetworkSimulator` is the main public entry point for the library.
/// It is responsible for creating, managing, and communicating with modem
/// instances.
pub struct ModemNetworkSimulator {
    modems: Mutex<HashMap<ModemId, Modem>>,
    pub(crate) callbacks: Arc<dyn NetworkCallbacks>,
    pub event_queue: Arc<Mutex<BinaryHeap<ScheduledEvent>>>,
    metrics: Arc<Metrics>,
    clock: Arc<dyn Clock>,
}

impl ModemNetworkSimulator {
    /// Creates a new `ModemNetworkSimulator` with a real system clock.
    pub fn new(callbacks: Arc<dyn NetworkCallbacks>) -> Arc<Self> {
        Self::new_with_clock(callbacks, Arc::new(SystemClock))
    }

    /// Creates a new `ModemNetworkSimulator` with a specific clock for testing.
    pub fn new_with_clock(
        callbacks: Arc<dyn NetworkCallbacks>,
        clock: Arc<dyn Clock>,
    ) -> Arc<Self> {
        Arc::new(Self {
            modems: Mutex::new(HashMap::new()),
            callbacks,
            event_queue: Arc::new(Mutex::new(BinaryHeap::new())),
            metrics: Arc::new(Metrics::default()),
            clock,
        })
    }

    /// Creates a new modem instance.
    pub fn new_modem(
        self: &Arc<Self>,
        id: ModemId,
        modem_callbacks: Arc<dyn Callbacks>,
    ) -> Result<(), ModemError> {
        self.new_modem_with_profile(id, modem_callbacks, None)
    }

    /// Creates a new modem instance with a specific SIM profile.
    pub fn new_modem_with_profile(
        self: &Arc<Self>,
        id: ModemId,
        modem_callbacks: Arc<dyn Callbacks>,
        profile: Option<crate::config::SimProfile>,
    ) -> Result<(), ModemError> {
        let mut modems = self.modems.lock().unwrap();
        if modems.contains_key(&id) {
            return Err(ModemError::DuplicateModemId(id));
        }
        let modem = crate::modem::ModemImpl::new(
            id,
            modem_callbacks,
            Arc::downgrade(self),
            profile.unwrap_or_default(),
        );
        modems.insert(id, modem);
        Ok(())
    }

    /// Removes a modem instance.
    pub fn remove_modem(&self, id: ModemId) {
        self.modems.lock().unwrap().remove(&id);
    }

    /// Sends an AT command to a modem instance.
    pub fn send_at_command(&self, id: ModemId, command: &[u8]) {
        log::debug!(
            "[Network] Received command for modem {}: {:?}",
            id,
            String::from_utf8_lossy(command)
        );
        self.metrics.at_commands_received.fetch_add(1, AtomicOrdering::Relaxed);
        let modem = self.get_modem(id);

        if let Some(modem) = modem {
            modem.receive_at_command(command);
        }
    }

    fn get_modems(&self) -> Vec<Modem> {
        self.modems.lock().unwrap().values().cloned().collect()
    }

    fn find_peer_modem<P>(&self, source_id: ModemId, predicate: P) -> Option<Modem>
    where
        P: Fn(&Modem) -> bool,
    {
        self.get_modems().into_iter().find(|m| m.id != source_id && predicate(m))
    }

    pub(crate) fn handle_command_action(&self, id: ModemId, action: CommandAction) {
        log::debug!("[Network] Handling action from {}: {:?}", id, action);
        match action {
            CommandAction::InitiateCall(phone_number) => {
                self.metrics.calls_initiated.fetch_add(1, AtomicOrdering::Relaxed);
                self.initiate_call(id, &phone_number);
            }
            CommandAction::InitiateCallAndHold(phone_number) => {
                self.metrics.calls_initiated.fetch_add(1, AtomicOrdering::Relaxed);
                if let Some(peer) = self.find_peer_modem(id, |m| m.call_service().is_active()) {
                    peer.call_service().receive_hold();
                }
                self.initiate_call(id, &phone_number);
                if let Some(modem) = self.get_modem(id) {
                    modem.callbacks.send_ok(id);
                }
            }
            CommandAction::SwapCalls(active_peer, held_peer) => {
                for modem in self.get_modems() {
                    if modem.id == active_peer {
                        modem.call_service().receive_hold();
                    } else if modem.id == held_peer {
                        modem.call_service().receive_resume();
                    }
                }
            }
            CommandAction::InitiateRemoteCall(phone_number) => {
                self.callbacks.on_new_remote_connection(id, phone_number.clone());
                self.initiate_call(id, &phone_number);
                if let Some(modem) = self.get_modem(id) {
                    modem.callbacks.send_ok(id);
                }
            }
            CommandAction::AnswerCall(answered_modem_id) => {
                self.metrics.calls_answered.fetch_add(1, AtomicOrdering::Relaxed);
                if let Some(caller) =
                    self.find_peer_modem(answered_modem_id, |m| m.call_service().is_dialing())
                {
                    if let Some(callee) = self.get_modem(answered_modem_id) {
                        caller.call_service().connect(
                            &caller.callbacks,
                            caller.id,
                            answered_modem_id,
                        );
                        callee.call_service().connect(&callee.callbacks, callee.id, caller.id);
                    }
                }
            }
            CommandAction::HangupCall(hung_up_modem_id) => {
                self.metrics.calls_hung_up.fetch_add(1, AtomicOrdering::Relaxed);
                for modem in self.get_modems() {
                    if !modem.call_service().is_idle() {
                        modem.call_service().receive_hangup();
                    }
                }
                self.callbacks.on_modem_hanged_up(hung_up_modem_id);
            }
            CommandAction::InitiateEmergencyCall => {} // No-op
            CommandAction::ReceiveSms(pdu) => {
                self.metrics.sms_sent.fetch_add(1, AtomicOrdering::Relaxed);
                if let Some(peer) = self.find_peer_modem(id, |_| true) {
                    let mut response = b"+CMT: ,".to_vec();
                    response.extend_from_slice(pdu.len().to_string().as_bytes());
                    response.extend_from_slice(b"\r\n");
                    response.extend_from_slice(&pdu);
                    response.extend_from_slice(b"\r\n");
                    peer.callbacks.send_at_response(peer.id, &response);
                }
            }
            CommandAction::ReceiveTextSms { to, text } => {
                self.metrics.sms_sent.fetch_add(1, AtomicOrdering::Relaxed);
                if let Some(peer) = self.find_peer_modem(id, |m| m.phone_number() == to) {
                    let mut response = format!(
                        "+CMT: \"{}\",\"\", \"25/08/03,16:56:00+00\"\r\n",
                        peer.phone_number()
                    )
                    .as_bytes()
                    .to_vec();
                    response.extend_from_slice(text.as_bytes());
                    response.extend_from_slice(b"\r\n");
                    peer.callbacks.send_at_response(peer.id, &response);
                }
            }
            CommandAction::None => {} // No-op
        }
    }

    /// Initiates a call from one modem to another.
    pub fn initiate_call(&self, caller_id: ModemId, phone_number: &str) {
        if let Some(target_modem) =
            self.find_peer_modem(caller_id, |m| m.phone_number() == phone_number)
        {
            target_modem.receive_at_command(b"RING\r\n");
            self.schedule_event(
                target_modem.id,
                CALL_RING_TIMEOUT,
                ModemEvent::CallRingTimeout { call_token: 1 },
            );
        }
    }

    /// Ticks the event loop.
    pub fn tick(&self) -> Option<Duration> {
        let mut event_queue = self.event_queue.lock().unwrap();
        let modems = self.get_modems();
        let modem_map: HashMap<ModemId, Modem> = modems.into_iter().map(|m| (m.id, m)).collect();

        let now = self.clock.now();

        while let Some(event) = event_queue.peek() {
            if event.when <= now {
                // rustc doesn't know that peek() and pop() are related, so we have to unwrap.
                let event = event_queue.pop().unwrap();
                if let Some(modem) = modem_map.get(&event.modem_id) {
                    modem.handle_event(event.event);
                }
            } else {
                break;
            }
        }

        event_queue.peek().map(|next_event| next_event.when - now)
    }

    /// Returns a snapshot of the current metrics.
    pub fn get_metrics(&self) -> MetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Returns the number of modem instances.
    pub fn get_modem_count(&self) -> usize {
        self.modems.lock().unwrap().len()
    }

    /// Returns a list of modem IDs.
    pub fn get_modem_ids(&self) -> Vec<ModemId> {
        self.modems.lock().unwrap().keys().cloned().collect()
    }

    /// Returns a modem instance.
    pub fn get_modem(&self, id: ModemId) -> Option<Modem> {
        self.modems.lock().unwrap().get(&id).cloned()
    }

    /// Returns the peer modem instance.
    pub fn get_peer(&self, id: ModemId) -> Option<Modem> {
        self.modems.lock().unwrap().values().find(|m| m.id != id).cloned()
    }

    pub fn external_echo_for_debug(&self, text: String) {
        log::debug!("[DEBUG] {}", text);
    }

    pub fn schedule_event(&self, modem_id: ModemId, delay: Duration, event: ModemEvent) {
        let mut event_queue = self.event_queue.lock().unwrap();
        let when = self.clock.now() + delay;
        event_queue.push(ScheduledEvent { when, modem_id, event });
    }
}

impl ModemNetworkInterface for ModemNetworkSimulator {
    fn add_modem(
        &self,
        _chip_id: ChipId,
        _callbacks: Arc<dyn ModemCallbacks>,
    ) -> Result<(), NetworkError> {
        unimplemented!();
    }
    fn remove_modem(&self, _chip_id: ChipId) -> Result<(), NetworkError> {
        unimplemented!();
    }
    fn send_data(&self, _chip_id: ChipId, _data: &[u8]) -> Result<(), NetworkError> {
        unimplemented!();
    }
    fn tick(&self) {
        unimplemented!();
    }
    fn get_modem_info(&self, _chip_id: ChipId) -> Result<Chip, NetworkError> {
        unimplemented!();
    }
}
