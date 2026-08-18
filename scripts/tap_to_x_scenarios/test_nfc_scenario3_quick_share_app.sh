#!/bin/bash
# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0
#
# Tap-to-X Scenario 3: Quick Share Sending Surface Verification

set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
ADB="${ANDROID_SDK_ROOT:-${HOME}/Library/Android/sdk}/platform-tools/adb"
NETSIM_BIN="${ANDROID_SDK_ROOT:-${HOME}/Library/Android/sdk}/emulator/netsim"

SENDER="emulator-5554"
RECEIVER="emulator-5556"

echo "=========================================================="
echo " Starting Tap-to-X Scenario 3: Quick Share Sending Surface"
echo " Sender:   ${SENDER} (Pixel 10)"
echo " Receiver: ${RECEIVER} (Pixel 10 (2))"
echo "=========================================================="

# 1. Reset both devices to Home screen and clear logcats
echo "[1/5] Resetting both devices to Home screen & clearing logcats..."
"${ADB}" -s "${SENDER}" shell input keyevent KEYCODE_HOME || true
"${ADB}" -s "${RECEIVER}" shell input keyevent KEYCODE_HOME || true
"${ADB}" -s "${SENDER}" logcat -c
"${ADB}" -s "${RECEIVER}" logcat -c

# 2. Position devices out of NFC range (1.0m)
echo "[2/5] Moving devices apart to 1.0m..."
"${NETSIM_BIN}" move "Pixel 10" 1.0 0.0 0.0
sleep 2

# 3. Launch Quick Share App directly on Sender
echo "[3/5] Launching Quick Share / Nearby Sharing on Sender..."
"${ADB}" -s "${SENDER}" shell "am start -n com.google.android.gms/.nearby.sharing.main.MainActivity" || true
sleep 3

# 4. Trigger Physical NFC Tap Proximity (0.02m)
echo "[4/5] Moving Pixel 10 to 0.02m (Physical Tap Contact)..."
"${NETSIM_BIN}" move "Pixel 10" 0.02 0.0 0.0

# 5. Allow Autonomous NFC Carrier Handshake
echo "[5/5] Waiting for Quick Share NFC handshake & BLE handover..."
for i in {1..8}; do
    echo "  Holding proximity (contact time: ${i}s)..."
    sleep 1
done

echo ""
echo "=========================================================="
echo " Scenario 3 Logcat Analysis (Sender):"
echo "=========================================================="
"${ADB}" -s "${SENDER}" logcat -d -s NearbySharing:V GestureExchange:V NfcService:D | tail -n 25 || true

echo ""
echo "=========================================================="
echo " Scenario 3 Logcat Analysis (Receiver):"
echo "=========================================================="
"${ADB}" -s "${RECEIVER}" logcat -d -s NearbySharing:V GestureExchange:V NfcService:D | tail -n 25 || true

echo ""
echo "=========================================================="
echo " Final Radio Statistics:"
echo "=========================================================="
"${NETSIM_BIN}" devices
echo "==========================================================="
echo " Scenario 3 Verification Complete!"
echo "==========================================================="
