#!/bin/bash
# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0
#
# Tap-to-X Scenario 2: System Share Sheet Tap-To-Share Verification

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
echo " Starting NFC Tap-to-Share (Scenario 2) Verification"
echo " Sender:   ${SENDER}"
echo " Receiver: ${RECEIVER}"
echo "=========================================================="

# 1. Clear logcats and prepare clean Home screen
echo "[1/5] Clearing logcats and resetting BOTH devices to Home page..."
"${ADB}" -s "${SENDER}" logcat -c || true
"${ADB}" -s "${RECEIVER}" logcat -c || true

"${ADB}" -s "${SENDER}" shell input keyevent KEYCODE_WAKEUP || true
"${ADB}" -s "${SENDER}" shell wm dismiss-keyguard || true
"${ADB}" -s "${SENDER}" shell input keyevent KEYCODE_HOME
"${ADB}" -s "${RECEIVER}" shell input keyevent KEYCODE_WAKEUP || true
"${ADB}" -s "${RECEIVER}" shell wm dismiss-keyguard || true
"${ADB}" -s "${RECEIVER}" shell input keyevent KEYCODE_HOME
sleep 2

# 2. Reset devices to 1.0m (out of NFC range) and wait for FastInitiation warming to end
echo "[2/5] Moving devices apart to 1.0m and waiting for FastInitiation warmup..."
DEV_SENDER=$("${NETSIM_BIN}" devices 2>/dev/null | grep -E "^(P10|Pixel 10|Pixel_10)[[:space:]]" | head -n 1 | awk -F'  +' '{print $1}' || true)
DEV_RECEIVER=$("${NETSIM_BIN}" devices 2>/dev/null | grep -E "^(P10_2|Pixel 10 \(2\)|Pixel_10_2)[[:space:]]" | head -n 1 | awk -F'  +' '{print $1}' || true)
DEV_SENDER="${DEV_SENDER:-Pixel 10}"
DEV_RECEIVER="${DEV_RECEIVER:-Pixel 10 (2)}"
echo "Identified Netsim Devices: Sender=${DEV_SENDER}, Receiver=${DEV_RECEIVER}"
"${NETSIM_BIN}" move "${DEV_SENDER}" 1.0 0.0 0.0
"${NETSIM_BIN}" move "${DEV_RECEIVER}" 0.0 0.0 0.0
sleep 6

# 3. Launch System Share Sheet on Sender (activates GestureExchange Reader Mode)
echo "[3/5] Launching System Share Sheet on Sender..."
"${ADB}" -s "${SENDER}" shell "am start -a android.intent.action.CHOOSER --eu android.intent.extra.INTENT 'intent:#Intent;action=android.intent.action.SEND;type=text/plain;S.android.intent.extra.TEXT=GestureExchange%20Tap-to-Share%20Verification;end'"

echo "Waiting for GMS TapToShare Reader Mode to register (flags: 385)..."
for i in {1..10}; do
    if "${ADB}" -s "${SENDER}" logcat -d -s NfcService | grep -q "flags: 385"; then
        echo "GMS Reader Mode registered successfully!"
        break
    fi
    sleep 0.5
done
sleep 1

# 4. Trigger Physical NFC Tap Proximity (0.02m)
echo "[4/5] Moving ${DEV_SENDER} to 0.02m (Physical Tap Contact)..."
"${NETSIM_BIN}" move "${DEV_SENDER}" 0.02 0.0 0.0

# 5. Allow Autonomous NFC Carrier Discovery & Handshake to complete
echo "[5/5] Waiting for autonomous NFC polling and observe mode handover..."
for i in {1..12}; do
    echo "  Holding proximity (contact time: ${i}s)..."
    sleep 1
done

echo "=========================================================="
echo " GestureExchange Logcat Analysis (Sender):"
echo "=========================================================="
"${ADB}" -s "${SENDER}" logcat -d -s GestureExchange NfcService HostEmulationManager NearbySharing | tail -n 30 || true

echo ""
echo "=========================================================="
echo " GestureExchange Logcat Analysis (Receiver):"
echo "=========================================================="
"${ADB}" -s "${RECEIVER}" logcat -d -s GestureExchange NfcService HostEmulationManager NearbySharing | tail -n 30 || true

echo ""
echo "=========================================================="
echo " APDU Transceive Traces:"
echo "=========================================================="
echo "--- Sender APDUs ---"
"${ADB}" -s "${SENDER}" logcat -d | grep -iE "transceive|SELECT_PRIMARY_AID|A00000047609" | tail -n 15 || true
echo "--- Receiver APDUs ---"
"${ADB}" -s "${RECEIVER}" logcat -d | grep -iE "HostApdu|A00000047609|handleSelectAid" | tail -n 15 || true

echo ""
echo "=========================================================="
echo " Final Radio Statistics:"
echo "=========================================================="
"${NETSIM_BIN}" devices

# Strict assertion: Verify that APDU transceives / AID selection occurred on both devices
SENDER_APDU_COUNT=$("${ADB}" -s "${SENDER}" logcat -d | grep -iE "transceive|SELECT_PRIMARY_AID|A00000047609" | wc -l)
RECEIVER_APDU_COUNT=$("${ADB}" -s "${RECEIVER}" logcat -d | grep -iE "HostApdu|A00000047609|handleSelectAid" | wc -l)

echo "=========================================================="
if [ "${SENDER_APDU_COUNT}" -gt 0 ] && [ "${RECEIVER_APDU_COUNT}" -gt 0 ]; then
    echo " SUCCESS: NFC Tap-to-Share APDU exchange verified (${SENDER_APDU_COUNT} sender / ${RECEIVER_APDU_COUNT} receiver events)"
    echo "=========================================================="
else
    echo " ERROR: NFC Tap-to-Share failed: APDU exchange not detected on both devices (Sender: ${SENDER_APDU_COUNT}, Receiver: ${RECEIVER_APDU_COUNT})"
    echo "=========================================================="
    exit 1
fi

