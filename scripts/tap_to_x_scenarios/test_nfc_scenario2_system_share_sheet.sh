#!/bin/bash
# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0
#
# Tap-to-X Scenario 2: System Share Sheet Tap-To-Share Verification

set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../../.." && pwd)"
ADB="${ANDROID_SDK_ROOT:-${HOME}/Library/Android/sdk}/platform-tools/adb"
NETSIM_BIN="${ANDROID_SDK_ROOT:-${HOME}/Library/Android/sdk}/emulator/netsim"

SENDER="emulator-5554"
RECEIVER="emulator-5556"

echo "=========================================================="
echo " Starting NFC Tap-to-Share (Scenario 2) Verification"
echo " Sender:   ${SENDER} (Pixel 10)"
echo " Receiver: ${RECEIVER} (Pixel 10 (2))"
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
"${NETSIM_BIN}" move "Pixel 10" 1.0 0.0 0.0
"${NETSIM_BIN}" move "Pixel 10 (2)" 0.0 0.0 0.0
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
echo "[4/5] Moving Pixel 10 to 0.02m (Physical Tap Contact)..."
"${NETSIM_BIN}" move "Pixel 10" 0.02 0.0 0.0

# Query active NFC chip IDs
CHIP_IDS=($("${NETSIM_BIN}" nfc list | awk 'NR>2 {print $1}'))
SENDER_CHIP="${CHIP_IDS[0]}"
RECEIVER_CHIP="${CHIP_IDS[1]}"
echo "Detected NFC Chips: Sender=${SENDER_CHIP}, Receiver=${RECEIVER_CHIP}"

# 5. Allow Autonomous NFC Carrier Discovery & Handshake to complete
echo "[5/5] Waiting for autonomous NFC polling and observe mode handover..."
for i in {1..12}; do
    echo "  Holding proximity (contact time: ${i}s)..."
    sleep 1
done

# Capture Screenshots for verification
echo "Capturing verification screenshots..."
"${ADB}" -s "${SENDER}" shell screencap -p /sdcard/sender_s2_run.png
"${ADB}" -s "${SENDER}" pull /sdcard/sender_s2_run.png "${DIR}/scratch/sender_s2_run.png" >/dev/null 2>&1 || true

"${ADB}" -s "${RECEIVER}" shell screencap -p /sdcard/receiver_s2_run.png
"${ADB}" -s "${RECEIVER}" pull /sdcard/receiver_s2_run.png "${DIR}/scratch/receiver_s2_run.png" >/dev/null 2>&1 || true

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
"${ADB}" -s "${SENDER}" logcat -d | grep -iE "transceive|SELECT_PRIMARY_AID|A00000047609|GestureExchangeActivity" | tail -n 15 || true
echo "--- Receiver APDUs ---"
"${ADB}" -s "${RECEIVER}" logcat -d | grep -iE "HostApdu|A00000047609|ACTION_INTERNAL_START" | tail -n 15 || true

echo ""
echo "=========================================================="
echo " Final Radio Statistics:"
echo "=========================================================="
"${NETSIM_BIN}" devices

echo "=========================================================="
echo " GestureExchange Tap-to-Share Analysis Complete!"
echo "=========================================================="

