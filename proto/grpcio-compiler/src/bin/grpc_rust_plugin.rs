// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

extern crate grpcio_compiler;

use grpcio_compiler::codegen;

fn main() {
    codegen::protoc_gen_grpc_rust_main();
}
