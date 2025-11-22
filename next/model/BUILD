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

load("@rules_rust//rust:defs.bzl", "rust_clippy", "rust_doc", "rust_doc_test", "rust_library", "rust_test")

package(default_visibility = ["//visibility:public"])

NETSIM_API_DEPS = [
    "@bytes",
    "@chrono",
    "@futures",
    "@log",
    "@serde",
    "@thiserror",
    "@tokio",
    "@tokio-stream",
]

rust_library(
    name = "model",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "netsim_model",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
        "@thiserror-impl",
    ],
    deps = NETSIM_API_DEPS,
)

rust_test(
    name = "tests",
    srcs = glob(["src/**/*.rs"]),
    crate_root = "src/lib.rs",
    edition = "2021",
    proc_macro_deps = [
        "@async-trait",
        "@thiserror-impl",
    ],
    tags = ["general_tests"],
    deps = NETSIM_API_DEPS,
)

rust_clippy(
    name = "clippy",
    deps = [":model"],
)

rust_doc(
    name = "doc",
    crate = ":model",
)

rust_doc_test(
    name = "doc_test",
    crate = ":model",
)
