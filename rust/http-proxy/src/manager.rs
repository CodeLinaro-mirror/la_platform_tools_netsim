// Copyright 2024 Google LLC
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

use crate::ProxyConfig;
use libslirp_rs::libslirp::{ProxyConnect, ProxyManager};
use log::info;
use std::net::SocketAddr;

pub struct Manager {}

impl Manager {
    pub fn new(_config: ProxyConfig) -> Self {
        Manager {}
    }
}

impl ProxyManager for Manager {
    fn try_connect(
        &self,
        sockaddr: SocketAddr,
        connect_id: usize,
        _connect_func: Box<dyn ProxyConnect>,
    ) -> bool {
        info!("try_connect: sockaddr:{:?} connect_id {}", sockaddr, connect_id);
        false
    }

    fn remove(&self, connect_id: usize) {
        info!("remove connect id {}", connect_id);
    }
}
