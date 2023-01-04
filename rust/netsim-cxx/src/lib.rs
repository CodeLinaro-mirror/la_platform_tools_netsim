//! Netsim cxx libraries.

mod frontend_http_server;
mod ranging;

use crate::frontend_http_server::run_frontend_http_server;
use crate::ranging::*;

#[cxx::bridge(namespace = "netsim")]
mod ffi {

    extern "Rust" {

        #[cxx_name = "RunFrontendHttpServer"]
        fn run_frontend_http_server();

        // Ranging

        #[cxx_name = "DistanceToRssi"]
        fn distance_to_rssi(tx_power: i8, distance: f32) -> i8;
    }

    extern "C++" {}
}
