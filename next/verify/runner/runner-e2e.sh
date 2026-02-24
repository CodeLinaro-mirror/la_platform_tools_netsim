#!/bin/bash
# Wrapper to run android networking end-to-end test suite

RUNNER_BIN=$1
APK_PATH=$2
NETSIM_BIN=$3
DRY_RUN=$4

echo "[runner@host] Running ntest run"
echo "[runner@host] APK: $APK_PATH"
echo "[runner@host] Netsimd: $NETSIM_BIN"

# Run ntest run command
$RUNNER_BIN run \
    --apk-path "$APK_PATH" \
    --netsim-path "$NETSIM_BIN" \
    $DRY_RUN
