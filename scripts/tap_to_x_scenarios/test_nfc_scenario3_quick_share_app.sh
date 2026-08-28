#!/bin/bash
# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0
#
# Tap-to-X Scenario 3: Quick Share Sending Surface Verification

set -euo pipefail

if [[ "$(uname)" == "Darwin" ]]; then
    DEFAULT_SDK="${HOME}/Library/Android/sdk"
else
    DEFAULT_SDK="${HOME}/Android/Sdk"
fi
SDK_DIR="${ANDROID_SDK_ROOT:-${DEFAULT_SDK}}"
ADB="${SDK_DIR}/platform-tools/adb"
NETSIM_BIN="${SDK_DIR}/emulator/netsim"

SENDER="emulator-5554"
RECEIVER="emulator-5556"

echo "=========================================================="
echo " Starting Tap-to-X Scenario 3: Quick Share Sending Surface"
echo " Sender:   ${SENDER}"
echo " Receiver: ${RECEIVER}"
echo "=========================================================="

# 1. Reset both devices to Home screen and clear logcats
echo "[1/5] Resetting both devices to Home screen & clearing logcats..."
"${ADB}" -s "${SENDER}" shell input keyevent KEYCODE_HOME || true
"${ADB}" -s "${RECEIVER}" shell input keyevent KEYCODE_HOME || true
"${ADB}" -s "${SENDER}" logcat -c
"${ADB}" -s "${RECEIVER}" logcat -c

# 2. Position devices out of NFC range (1.0m)
echo "[2/5] Moving devices apart to 1.0m..."
DEV_SENDER=$("${NETSIM_BIN}" devices 2>/dev/null | grep -E "^(P10|Pixel 10|Pixel_10)[[:space:]]" | head -n 1 | awk -F'  +' '{print $1}' || true)
DEV_RECEIVER=$("${NETSIM_BIN}" devices 2>/dev/null | grep -E "^(P10_2|Pixel 10 \(2\)|Pixel_10_2)[[:space:]]" | head -n 1 | awk -F'  +' '{print $1}' || true)
DEV_SENDER="${DEV_SENDER:-Pixel 10}"
DEV_RECEIVER="${DEV_RECEIVER:-Pixel 10 (2)}"
echo "Identified Netsim Devices: Sender=${DEV_SENDER}, Receiver=${DEV_RECEIVER}"
"${NETSIM_BIN}" move "${DEV_SENDER}" 1.0 0.0 0.0
"${NETSIM_BIN}" move "${DEV_RECEIVER}" 0.0 0.0 0.0
sleep 2

# 3. Launch Quick Share App directly on Sender
echo "[3/5] Launching Quick Share / Nearby Sharing on Sender..."
"${ADB}" -s "${SENDER}" shell "am start -n com.google.android.gms/.nearby.sharing.main.MainActivity" || true
sleep 3

# 4. Trigger Physical NFC Tap Proximity (0.02m)
echo "[4/5] Moving ${DEV_SENDER} to 0.02m (Physical Tap Contact)..."
"${NETSIM_BIN}" move "${DEV_SENDER}" 0.02 0.0 0.0

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

# Strict assertion: Verify that NFC / APDU transceives actually occurred on both devices
SENDER_EVENTS=$("${ADB}" -s "${SENDER}" logcat -d | grep -iE "transceive|SELECT_PRIMARY_AID|A00000047609" | wc -l)
RECEIVER_EVENTS=$("${ADB}" -s "${RECEIVER}" logcat -d | grep -iE "HostApdu|A00000047609|handleSelectAid|ContactExchangeActivity" | wc -l)

echo "=========================================================="
if [ "${SENDER_EVENTS}" -gt 0 ] && [ "${RECEIVER_EVENTS}" -gt 0 ]; then
    echo " SUCCESS: Quick Share NFC handshake verified (${SENDER_EVENTS} sender / ${RECEIVER_EVENTS} receiver events)"
    echo "=========================================================="
else
    echo " ERROR: Quick Share NFC handshake failed: APDU exchange not detected on both devices (Sender: ${SENDER_EVENTS}, Receiver: ${RECEIVER_EVENTS})"
    echo "=========================================================="
    exit 1
fi
