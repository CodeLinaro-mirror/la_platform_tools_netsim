// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

// UWB API stats tracking rebase.

use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use netsim_proto::stats::{RangingSessionStats, UwbApiStats, UwbManagerStats};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UwbApi {
    Open,
    Close,
    Start,
    Stop,
    Reconfigure,
    SessionGetAppConfig,
    SendData,
    OnDataReceived,
    AddControlee,
    RemoveControlee,
    Count,
}

pub const UWB_API_COUNT: usize = UwbApi::Count as usize;

#[derive(Debug)]
struct UwbApiArray {
    data: [AtomicU32; UWB_API_COUNT],
}

impl Default for UwbApiArray {
    fn default() -> Self {
        Self { data: std::array::from_fn(|_| AtomicU32::new(0)) }
    }
}

#[derive(Clone, Debug, Default)]
pub struct UwbStats {
    uwb_apis: Arc<UwbApiArray>,
}

impl UwbStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn incr(&self, api: UwbApi) {
        self.uwb_apis.data[api as usize].fetch_add(1, Ordering::Relaxed);
    }

    pub fn get(&self, api: UwbApi) -> u32 {
        self.uwb_apis.data[api as usize].load(Ordering::Relaxed)
    }

    pub fn to_proto(&self) -> netsim_proto::stats::UwbApiStats {
        let saturate = |val: u32| -> i32 { val.try_into().unwrap_or(i32::MAX) };

        let mut uwb = UwbApiStats::new();

        let mut uwb_manager = UwbManagerStats::new();
        uwb_manager.open_ranging_session = Some(saturate(self.get(UwbApi::Open)));
        uwb.uwb_manager = netsim_proto::protobuf::MessageField::some(uwb_manager);

        let mut ranging_session = RangingSessionStats::new();
        ranging_session.close = Some(saturate(self.get(UwbApi::Close)));
        ranging_session.start = Some(saturate(self.get(UwbApi::Start)));
        ranging_session.stop = Some(saturate(self.get(UwbApi::Stop)));
        ranging_session.reconfigure = Some(saturate(self.get(UwbApi::Reconfigure)));
        ranging_session.session_get_app_config =
            Some(saturate(self.get(UwbApi::SessionGetAppConfig)));
        ranging_session.send_data = Some(saturate(self.get(UwbApi::SendData)));
        ranging_session.on_data_received = Some(saturate(self.get(UwbApi::OnDataReceived)));
        ranging_session.add_controlee = Some(saturate(self.get(UwbApi::AddControlee)));
        ranging_session.remove_controlee = Some(saturate(self.get(UwbApi::RemoveControlee)));
        uwb.ranging_session = netsim_proto::protobuf::MessageField::some(ranging_session);

        uwb
    }
}
