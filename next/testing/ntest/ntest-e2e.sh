#!/bin/bash
# Wrapper to run android networking end-to-end test suite

NTEST_BIN=$1
APK_PATH=$2
NETSIM_BIN=$3
DRY_RUN=$4

echo "[ntest@host] Running ntest run"
echo "[ntest@host] APK: $APK_PATH"
echo "[ntest@host] Netsimd: $NETSIM_BIN"

# Run ntest run command
$NTEST_BIN run \
    --apk-path "$APK_PATH" \
    --netsim-path "$NETSIM_BIN" \
    $DRY_RUN
