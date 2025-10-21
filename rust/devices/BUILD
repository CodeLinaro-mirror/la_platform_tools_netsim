# Copyright 2023 The Android Open Source Project
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law of or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

load("@rules_rust//rust:defs.bzl", "rust_clippy", "rust_doc", "rust_library", "rust_test")

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "devices",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "devices",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
        "@thiserror-impl",
    ],
    deps = [
        "//rust/netsim-api",
        "//rust/proto-cf",
        "@bytes",
        "@chrono",
        "@futures",
        "@log",
        "@protobuf-rust",
        "@tokio",
        "@tokio-stream",
    ],
)

rust_test(
    name = "unit_tests",
    crate = ":devices",
    tags = ["general_tests"],
)

rust_test(
    name = "tests",
    srcs = glob(["tests/**/*.rs"]),
    crate_root = "tests/mod.rs",
    proc_macro_deps = [
        "@async-trait",
    ],
    rustc_flags = [
        "-C",
        "link-arg=-lc",
    ],
    tags = ["general_tests"],
    deps = [
        ":devices",
        "//rust/netsim-api",
        "//rust/proto-cf",
        "@bytes",
        "@chrono",
        "@futures",
        "@log",
        "@protobuf-rust",
        "@thiserror",
        "@tokio",
        "@tokio-stream",
    ],
)

rust_clippy(
    name = "clippy",
    deps = [":devices"],
)

rust_doc(
    name = "doc",
    crate = ":devices",
)
