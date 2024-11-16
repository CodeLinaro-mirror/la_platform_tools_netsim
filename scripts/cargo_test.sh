#!/bin/bash
# Copyright 2024 The Android Open Source Project
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# Get the directory of the script
REPO=$(dirname "$0")/../../..

# Get the Rust version, package, and objs path from arguments
RUST_VERSION="$1"
RUST_PKG="$2"
OUT_PATH="$3"

# The possible values are "linux" and "darwin".
OS=$(uname | tr '[:upper:]' '[:lower:]')

# Set environment variables
export CARGO_HOME=$OUT_PATH/rust/.cargo
export OBJS_PATH=$OUT_PATH
# Paths to pdl generated packets files
PDL_PATH=$OUT_PATH/pdl/pdl_gen
export MAC80211_HWSIM_PACKETS_PREBUILT=$PDL_PATH/mac80211_hwsim_packets.rs
export IEEE80211_PACKETS_PREBUILT=$PDL_PATH/ieee80211_packets.rs
export LLC_PACKETS_PREBUILT=$PDL_PATH/llc_packets.rs
export NETLINK_PACKETS_PREBUILT=$PDL_PATH/netlink_packets.rs

# Build the package
ninja -C $OUT_PATH $RUST_PKG

if [[ "$OS" == "darwin" ]]; then
  export DYLD_FALLBACK_LIBRARY_PATH=$OUT_PATH/lib64
else
  export LD_LIBRARY_PATH=$OUT_PATH/lib64
fi

# Run the cargo command
# TODO(360874898): prebuilt rust toolchain for darwin-aarch64 is supported from 1.77.1
if [[ "$OS" == "darwin" && $(uname -m) == "arm64" ]]; then
  cargo test -vv --package $RUST_PKG --manifest-path $REPO/tools/netsim/rust/Cargo.toml -- --nocapture
else
  $REPO/prebuilts/rust/$OS-x86/$RUST_VERSION/bin/cargo test -vv --package $RUST_PKG --manifest-path $REPO/tools/netsim/rust/Cargo.toml -- --nocapture
fi
