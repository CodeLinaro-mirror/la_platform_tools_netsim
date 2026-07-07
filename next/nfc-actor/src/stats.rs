// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::sync::atomic::{AtomicU64, Ordering};

/// Stats for the NFC Actor.
///
/// Tracks error counts, packet counts, and Casimir RF interactions.
#[derive(Debug, Default)]
pub struct NfcStats {
    pub nci_errors: AtomicU64,
    pub casimir_errors: AtomicU64,
    pub rf_errors: AtomicU64,
    pub other_errors: AtomicU64,

    pub nci_commands_rx: AtomicU64,
    pub nci_responses_tx: AtomicU64,
    pub nci_notifications_tx: AtomicU64,
    pub nci_data_rx: AtomicU64,
    pub nci_data_tx: AtomicU64,
    pub rf_taps_tx: AtomicU64,
    pub rf_taps_rx: AtomicU64,
    pub card_emulation_count: AtomicU64,
    pub tag_emulation_count: AtomicU64,
}

/// Stats for the NFC frontend gRPC service (`NfcService`).
///
/// Tracks invocation counts across the frontend gRPC service endpoints
/// (`get_status`, `set_power`, `poll`, `send_apdu`).
#[derive(Debug, Default)]
pub struct NfcServiceStats {
    /// Invocations of `NfcService.GetStatus`.
    pub get_status: AtomicU64,
    /// Invocations of `NfcService.SetPower`.
    pub set_power: AtomicU64,
    /// Invocations of `NfcService.Poll`.
    pub poll: AtomicU64,
    /// Invocations of `NfcService.SendApdu`.
    pub send_apdu: AtomicU64,
}

fn saturate_i32(val: u64) -> i32 {
    std::cmp::min(val, i32::MAX as u64) as i32
}

fn saturate_u32(val: u64) -> u32 {
    std::cmp::min(val, u32::MAX as u64) as u32
}

impl NfcStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn incr_nci_commands_rx(&self) {
        self.nci_commands_rx.fetch_add(1, Ordering::Relaxed);
    }
    pub fn incr_nci_responses_tx(&self) {
        self.nci_responses_tx.fetch_add(1, Ordering::Relaxed);
    }
    pub fn incr_nci_notifications_tx(&self) {
        self.nci_notifications_tx.fetch_add(1, Ordering::Relaxed);
    }
    pub fn incr_nci_data_rx(&self) {
        self.nci_data_rx.fetch_add(1, Ordering::Relaxed);
    }
    pub fn incr_nci_data_tx(&self) {
        self.nci_data_tx.fetch_add(1, Ordering::Relaxed);
    }
    pub fn incr_rf_taps_tx(&self) {
        self.rf_taps_tx.fetch_add(1, Ordering::Relaxed);
    }
    pub fn incr_rf_taps_rx(&self) {
        self.rf_taps_rx.fetch_add(1, Ordering::Relaxed);
    }
    pub fn incr_card_emulation_count(&self) {
        self.card_emulation_count.fetch_add(1, Ordering::Relaxed);
    }
    pub fn incr_tag_emulation_count(&self) {
        self.tag_emulation_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn incr_nci_error(&self) {
        self.nci_errors.fetch_add(1, Ordering::Relaxed);
    }
    pub fn incr_casimir_error(&self) {
        self.casimir_errors.fetch_add(1, Ordering::Relaxed);
    }
    pub fn incr_rf_error(&self) {
        self.rf_errors.fetch_add(1, Ordering::Relaxed);
    }
    pub fn incr_other_error(&self) {
        self.other_errors.fetch_add(1, Ordering::Relaxed);
    }

    pub fn to_proto(&self) -> netsim_proto::stats::NfcStats {
        let mut proto = netsim_proto::stats::NfcStats::new();

        proto.set_nci_errors(saturate_i32(self.nci_errors.load(Ordering::Relaxed)));
        proto.set_casimir_errors(saturate_i32(self.casimir_errors.load(Ordering::Relaxed)));
        proto.set_rf_errors(saturate_i32(self.rf_errors.load(Ordering::Relaxed)));
        proto.set_other_errors(saturate_i32(self.other_errors.load(Ordering::Relaxed)));

        proto.set_nci_commands_rx(saturate_i32(self.nci_commands_rx.load(Ordering::Relaxed)));
        proto.set_nci_responses_tx(saturate_i32(self.nci_responses_tx.load(Ordering::Relaxed)));
        proto.set_nci_notifications_tx(saturate_i32(
            self.nci_notifications_tx.load(Ordering::Relaxed),
        ));
        proto.set_nci_data_rx(saturate_i32(self.nci_data_rx.load(Ordering::Relaxed)));
        proto.set_nci_data_tx(saturate_i32(self.nci_data_tx.load(Ordering::Relaxed)));
        proto.set_rf_taps_tx(saturate_i32(self.rf_taps_tx.load(Ordering::Relaxed)));
        proto.set_rf_taps_rx(saturate_i32(self.rf_taps_rx.load(Ordering::Relaxed)));
        proto.set_card_emulation_count(saturate_i32(
            self.card_emulation_count.load(Ordering::Relaxed),
        ));
        proto.set_tag_emulation_count(saturate_i32(
            self.tag_emulation_count.load(Ordering::Relaxed),
        ));

        proto
    }
}

impl NfcServiceStats {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn incr_get_status_count(&self) {
        self.get_status.fetch_add(1, Ordering::Relaxed);
    }
    pub fn incr_set_power_count(&self) {
        self.set_power.fetch_add(1, Ordering::Relaxed);
    }
    pub fn incr_poll_count(&self) {
        self.poll.fetch_add(1, Ordering::Relaxed);
    }
    pub fn incr_send_apdu_count(&self) {
        self.send_apdu.fetch_add(1, Ordering::Relaxed);
    }
    pub fn to_proto(&self) -> netsim_proto::stats::NfcServiceStats {
        let mut proto = netsim_proto::stats::NfcServiceStats::new();
        proto.set_get_status(saturate_u32(self.get_status.load(Ordering::Relaxed)));
        proto.set_set_power(saturate_u32(self.set_power.load(Ordering::Relaxed)));
        proto.set_poll(saturate_u32(self.poll.load(Ordering::Relaxed)));
        proto.set_send_apdu(saturate_u32(self.send_apdu.load(Ordering::Relaxed)));
        proto
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nfc_stats_increments() {
        let stats = NfcStats::new();
        stats.incr_nci_commands_rx();
        stats.incr_nci_responses_tx();
        stats.incr_nci_responses_tx();
        stats.incr_nci_data_rx();
        stats.incr_nci_error();

        let proto = stats.to_proto();
        assert_eq!(proto.nci_commands_rx(), 1);
        assert_eq!(proto.nci_responses_tx(), 2);
        assert_eq!(proto.nci_data_rx(), 1);
        assert_eq!(proto.nci_errors(), 1);
        assert_eq!(proto.nci_data_tx(), 0);
    }

    #[test]
    fn test_nfc_stats_saturation() {
        let stats = NfcStats::new();
        stats.nci_errors.store(i32::MAX as u64 + 100, Ordering::Relaxed);
        let proto = stats.to_proto();
        assert_eq!(proto.nci_errors(), i32::MAX);
    }

    #[test]
    fn test_nfc_service_stats_increments() {
        let service_stats = NfcServiceStats::new();
        service_stats.incr_get_status_count();
        service_stats.incr_set_power_count();
        service_stats.incr_poll_count();
        service_stats.incr_poll_count();

        let proto = service_stats.to_proto();
        assert_eq!(proto.get_status(), 1);
        assert_eq!(proto.set_power(), 1);
        assert_eq!(proto.poll(), 2);
        assert_eq!(proto.send_apdu(), 0);
    }
}
