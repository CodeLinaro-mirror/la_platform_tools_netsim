//
//  Copyright 2022 Google, Inc.
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at:
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
use std::path::Path;

fn main() {
    let _build = cxx_build::bridge("src/lib.rs");
    println!("cargo:rerun-if-changed=src/lib.rs");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let generated_cpp_file = Path::new(&out_dir)
        .join("cxxbridge")
        .join("sources")
        .join("frontend-client-cxx")
        .join("src")
        .join("lib.rs.cc");
    let generated_header_file = Path::new(&out_dir)
        .join("cxxbridge")
        .join("include")
        .join("frontend-client-cxx")
        .join("src")
        .join("lib.rs.h");
    std::fs::copy(
        &generated_header_file,
        &Path::new(&manifest_dir).join("cxx").join("frontend_client_cxx_generated.h"),
    )
    .unwrap();
    std::fs::copy(
        &generated_cpp_file,
        &Path::new(&manifest_dir).join("cxx").join("frontend_client_cxx_generated.cc"),
    )
    .unwrap();
}
