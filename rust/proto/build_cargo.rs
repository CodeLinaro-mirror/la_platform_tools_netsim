//  Copyright 2021 The Android Open Source Project
//
// SPDX-License-Identifier: Apache-2.0

use std::env;
use std::path::{Path, PathBuf};

fn paths_to_strs<P: AsRef<Path>>(paths: &[P]) -> Vec<&str> {
    paths.iter().map(|p| p.as_ref().as_os_str().to_str().unwrap()).collect()
}

fn main() {
    // Proto root is //tools/netsim/proto/netsim
    let proto_root = match env::var("PLATFORM_SUBDIR") {
        Ok(dir) => PathBuf::from(dir).join("tools/netsim"),
        // Currently at //tools/netsim/rust/proto
        Err(_) => PathBuf::from(env::current_dir().unwrap()).join("../..").canonicalize().unwrap(),
    };

    // Generate protobuf output
    let proto_dir = proto_root.join("proto");
    let rootcanal_dir = proto_root.join("../../tools/rootcanal/proto");
    let proto_input_files = [
        rootcanal_dir.join("rootcanal/configuration.proto"),
        proto_dir.join("netsim/packet_streamer.proto"),
        proto_dir.join("netsim/hci_packet.proto"),
        proto_dir.join("netsim/startup.proto"),
        proto_dir.join("netsim/model.proto"),
        proto_dir.join("netsim/frontend.proto"),
        proto_dir.join("netsim/common.proto"),
        proto_dir.join("netsim/config.proto"),
        proto_dir.join("netsim/stats.proto"),
    ];
    let proto_include_dirs = [proto_dir.clone(), rootcanal_dir.clone()];
    let proto_out_dir = proto_root.join("rust/proto/src");

    println!("cargo:warning=proto_outdir={:?}", proto_out_dir);

    let customize = protobuf_codegen::Customize::default().gen_mod_rs(false);

    protobuf_codegen::Codegen::new()
        .out_dir(proto_out_dir.as_os_str().to_str().unwrap())
        .inputs(&paths_to_strs(&proto_input_files))
        .includes(&paths_to_strs(&proto_include_dirs))
        .customize(customize)
        .run()
        .expect("protoc");
}
