#!/bin/bash -eu

# Get the directory of the script
REPO=$(dirname "$0")/../../..

# The possible values are "linux" and "darwin".
OS=$(uname | tr '[:upper:]' '[:lower:]')

OUT_PATH="$1"
RUST_VERSION="$2"
CLIPPY_FLAGS="$3"

source $REPO/tools/netsim/scripts/cargo_env.sh $OUT_PATH

pushd $REPO/tools/netsim/rust
# Run the cargo command
# TODO(360874898): prebuilt rust toolchain for darwin-aarch64 is supported from 1.77.1
if [[ "$OS" == "darwin" && $(uname -m) == "arm64" ]]; then
  cargo clippy -- $CLIPPY_FLAGS
else
  $REPO/prebuilts/rust/$OS-x86/$RUST_VERSION/bin/cargo clippy -- $CLIPPY_FLAGS
fi
popd
