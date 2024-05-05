// Copyright 2023 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use futures::{channel::mpsc::UnboundedSender, sink::SinkExt};
use lazy_static::lazy_static;
use pica::{Handle, Pica};

use netsim_proto::model::chip::Radio as ProtoRadio;
use netsim_proto::model::Chip as ProtoChip;
use netsim_proto::stats::{netsim_radio_stats, NetsimRadioStats as ProtoRadioStats};

use crate::devices::chip::ChipIdentifier;
use crate::echip::packet::handle_response_rust;
use crate::uwb::ranging_estimator::{SharedState, UwbRangingEstimator};

use std::sync::{Arc, Mutex};
use std::thread;

use super::{EmulatedChip, SharedEmulatedChip};

// TODO(b/331267949): Construct Manager struct for each echip module
lazy_static! {
    static ref PICA_HANDLE_TO_STATE: SharedState = SharedState::new();
    static ref PICA: Arc<Mutex<Pica>> = Arc::new(Mutex::new(Pica::new(
        Box::new(UwbRangingEstimator::new(PICA_HANDLE_TO_STATE.clone())),
        None
    )));
    static ref PICA_RUNTIME: Arc<tokio::runtime::Runtime> =
        Arc::new(tokio::runtime::Runtime::new().unwrap());
}

/// Parameters for creating UWB chips
pub struct CreateParams {
    pub address: String,
}

/// UWB struct will keep track of pica_id
pub struct Uwb {
    chip_id: ChipIdentifier,
    pica_id: Handle,
    uci_stream_writer: UnboundedSender<Vec<u8>>,
    state: bool,
    tx_count: i32,
    rx_count: i32, // TODO(b/330788870): Increment rx_count after handle_response_rust
}

impl EmulatedChip for Uwb {
    fn handle_request(&self, packet: &[u8]) {
        // TODO(b/330788870): Increment tx_count
        self.uci_stream_writer
            .unbounded_send(packet.into())
            .expect("UciStream Receiver Disconnected");
    }

    fn reset(&self) {
        // TODO: Use AtomicBool, etc
        // self.state = true;
        // self.tx_count = 0;
        // self.rx_count = 0;
    }

    fn get(&self) -> ProtoChip {
        let mut chip_proto = ProtoChip::new();
        let uwb_proto = ProtoRadio {
            state: Some(self.state),
            tx_count: self.tx_count,
            rx_count: self.rx_count,
            ..Default::default()
        };
        chip_proto.mut_uwb().clone_from(&uwb_proto);
        chip_proto
    }

    fn patch(&self, _chip: &ProtoChip) {
        // TODO(b/330789027): Patch the state of UWB chip
        log::info!("Patch Uwb Chip for chip_id: {}", self.chip_id);
    }

    fn remove(&self) {
        PICA_HANDLE_TO_STATE.remove(&self.pica_id);
    }

    fn get_stats(&self, duration_secs: u64) -> Vec<ProtoRadioStats> {
        let mut stats_proto = ProtoRadioStats::new();
        stats_proto.set_duration_secs(duration_secs);
        stats_proto.set_kind(netsim_radio_stats::Kind::UWB);
        let chip_proto = self.get();
        if chip_proto.has_uwb() {
            stats_proto.set_tx_count(chip_proto.uwb().tx_count);
            stats_proto.set_rx_count(chip_proto.uwb().rx_count);
        }
        vec![stats_proto]
    }
}

pub fn uwb_start() {
    // TODO: Provide TcpStream as UWB connector
    let _ = thread::Builder::new().name("pica_service".to_string()).spawn(move || {
        log::info!("PICA STARTED");
        let _guard = PICA_RUNTIME.enter();
        futures::executor::block_on(pica::run(&PICA))
    });
}

pub fn new(_create_params: &CreateParams, chip_id: ChipIdentifier) -> SharedEmulatedChip {
    let (uci_stream_sender, uci_stream_receiver) = futures::channel::mpsc::unbounded();
    let (uci_sink_sender, uci_sink_receiver) = futures::channel::mpsc::unbounded();
    let _guard = PICA_RUNTIME.enter();
    let pica_id = PICA
        .lock()
        .unwrap()
        .add_device(Box::pin(uci_stream_receiver), Box::pin(uci_sink_sender.sink_err_into()))
        .unwrap();
    PICA_HANDLE_TO_STATE.insert(pica_id, chip_id);
    let echip = Uwb {
        chip_id,
        pica_id,
        uci_stream_writer: uci_stream_sender,
        state: true,
        tx_count: 0,
        rx_count: 0,
    };

    // Thread for obtaining packet from pica and invoking handle_response_rust
    let _ =
        thread::Builder::new().name(format!("uwb_packet_response_{chip_id}")).spawn(move || {
            for packet in futures::executor::block_on_stream(uci_sink_receiver) {
                handle_response_rust(chip_id, packet.into());
            }
        });
    Arc::new(Box::new(echip))
}

#[cfg(test)]
mod tests {

    use super::*;

    fn new_uwb_shared_echip() -> SharedEmulatedChip {
        new(&CreateParams { address: "test".to_string() }, 0)
    }

    #[test]
    fn test_uwb_get() {
        let shared_echip = new_uwb_shared_echip();
        assert!(shared_echip.get().has_uwb());
    }

    #[test]
    fn test_uwb_reset() {
        // TODO(b/330789027): Patch the state of UWB echip before reset
        let shared_echip = new_uwb_shared_echip();
        shared_echip.reset();
        let binding = shared_echip.get();
        let radio = binding.uwb();
        assert_eq!(radio.rx_count, 0);
        assert_eq!(radio.tx_count, 0);
        assert_eq!(radio.state, Some(true));
    }

    #[test]
    fn test_get_stats() {
        let shared_echip = new_uwb_shared_echip();
        let radio_stat_vec = shared_echip.get_stats(0);
        let radio_stat = radio_stat_vec.first().unwrap();
        assert_eq!(radio_stat.kind(), netsim_radio_stats::Kind::UWB);
        assert_eq!(radio_stat.duration_secs(), 0);
    }
}
