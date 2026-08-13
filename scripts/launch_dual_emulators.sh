#!/bin/bash
# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0
#
# Launch Netsim Daemon & Dual Headless Emulators with Host DNS, Nfc & netsimx Features

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SDK_DIR="${ANDROID_SDK_ROOT:-${HOME}/Library/Android/sdk}"
EMULATOR_BIN="${SDK_DIR}/emulator/emulator"
NETSIM_BIN="${SDK_DIR}/emulator/netsim"
NETSIMD_BIN="${SDK_DIR}/emulator/netsimdx"
ADB_BIN="${SDK_DIR}/platform-tools/adb"

echo "=========================================================="
echo " Starting Netsim Daemon & Dual Pixel 10 Emulators (Headless)"
echo " Features: Nfc, netsimx"
echo "=========================================================="

# 1. Clean up existing processes
echo "[1/4] Terminating existing emulator and netsimd instances..."
pkill -9 -f "qemu-system-aarch64" 2>/dev/null || true
pkill -9 -f "netsimd" 2>/dev/null || true
pkill -9 -f "netsimdx" 2>/dev/null || true
sleep 2

# 2. Start Netsim Daemon with Host DNS routing
echo "[2/4] Starting Netsim Daemon (${NETSIMD_BIN}) with --host-dns 8.8.8.8,8.8.4.4..."
"${NETSIMD_BIN}" --host-dns 8.8.8.8,8.8.4.4 > /tmp/netsimd_runtime.log 2>&1 &
sleep 2

# 3. Launch Dual Emulators
echo "[3/4] Launching Pixel_10 (5554) and Pixel_10_2 (5556) with Features..."
"${EMULATOR_BIN}" -avd Pixel_10 -port 5554 -no-window -no-snapshot-load -dns-server 8.8.8.8,8.8.4.4 -feature Nfc -feature netsimx > /tmp/emu_5554.log 2>&1 &
"${EMULATOR_BIN}" -avd Pixel_10_2 -port 5556 -no-window -no-snapshot-load -dns-server 8.8.8.8,8.8.4.4 -feature Nfc -feature netsimx > /tmp/emu_5556.log 2>&1 &

# 4. Wait for boot completion
echo "[4/4] Waiting for boot completion..."
START_TIME=$(date +%s)

while true; do
    CURRENT_TIME=$(date +%s)
    ELAPSED=$((CURRENT_TIME - START_TIME))

    if [ "$ELAPSED" -ge 90 ]; then
        echo "ERROR: Timed out waiting for emulators to boot after 90s!"
        exit 1
    fi

    B1=$("${ADB_BIN}" -s emulator-5554 shell getprop sys.boot_completed 2>/dev/null || echo "0")
    B2=$("${ADB_BIN}" -s emulator-5556 shell getprop sys.boot_completed 2>/dev/null || echo "0")

    if [[ "$B1" == "1" && "$B2" == "1" ]]; then
        echo "Both emulators booted successfully in ${ELAPSED}s!"
        break
    fi

    sleep 2
done

# Run Mobile Utilities & Flags setup
"${SCRIPT_DIR}/setup_mobile_utilities_flags.sh" emulator-5554 emulator-5556

echo "=========================================================="
echo " Netsim Topology:"
echo "=========================================================="
"${NETSIM_BIN}" devices

echo "=========================================================="
echo " Headless Emulators Ready with NFC Enabled!"
echo "=========================================================="
