#!/bin/bash
# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0
#
# Tap-to-X Scenario 1: Contacts Exchange Background Reader Mode Verification

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
echo " Starting Tap-to-X Scenario 1: Contacts Exchange"
echo " Sender:   ${SENDER}"
echo " Receiver: ${RECEIVER}"
echo " Specification: Both devices on Home Screen (Background Reader Mode)"
echo "=========================================================="

# 1. Reset both devices to Home screen and unlock
echo "[1/6] Resetting both devices to Home screen & unlocking..."
"${ADB}" -s "${SENDER}" shell input keyevent KEYCODE_WAKEUP || true
"${ADB}" -s "${SENDER}" shell wm dismiss-keyguard || true
"${ADB}" -s "${SENDER}" shell input keyevent KEYCODE_HOME || true

"${ADB}" -s "${RECEIVER}" shell input keyevent KEYCODE_WAKEUP || true
"${ADB}" -s "${RECEIVER}" shell wm dismiss-keyguard || true
"${ADB}" -s "${RECEIVER}" shell input keyevent KEYCODE_HOME || true

# 2. Clear logcat buffers & configure verbose logging
echo "[2/6] Configuring logging and clearing logcats..."
"${ADB}" -s "${SENDER}" shell setprop log.tag.GestureExchange VERBOSE || true
"${ADB}" -s "${RECEIVER}" shell setprop log.tag.GestureExchange VERBOSE || true
"${ADB}" -s "${SENDER}" logcat -c
"${ADB}" -s "${RECEIVER}" logcat -c

# 3. Position devices out of NFC range (1.0m)
echo "[3/6] Moving devices apart to 1.0m..."
DEV_SENDER=$("${NETSIM_BIN}" devices 2>/dev/null | grep -E "^(P10|Pixel 10|Pixel_10)[[:space:]]" | head -n 1 | awk -F'  +' '{print $1}' || true)
DEV_RECEIVER=$("${NETSIM_BIN}" devices 2>/dev/null | grep -E "^(P10_2|Pixel 10 \(2\)|Pixel_10_2)[[:space:]]" | head -n 1 | awk -F'  +' '{print $1}' || true)
DEV_SENDER="${DEV_SENDER:-Pixel 10}"
DEV_RECEIVER="${DEV_RECEIVER:-Pixel 10 (2)}"
echo "Identified Netsim Devices: Sender=${DEV_SENDER}, Receiver=${DEV_RECEIVER}"
"${NETSIM_BIN}" move "${DEV_SENDER}" 1.0 0.0 0.0
"${NETSIM_BIN}" move "${DEV_RECEIVER}" 0.0 0.0 0.0
sleep 2

# 4. Trigger Physical NFC Tap Proximity (0.02m) & Motion Sensor Tap
echo "[4/6] Moving ${DEV_SENDER} to 0.02m (Physical Tap Contact) & simulating tap motion..."
"${NETSIM_BIN}" move "${DEV_SENDER}" 0.02 0.0 0.0

# Simulate physical bump / tap acceleration pulse on both devices
"${ADB}" -s "${SENDER}" emu sensor set acceleration 0 25.0 5.0 2>/dev/null || true
"${ADB}" -s "${RECEIVER}" emu sensor set acceleration 0 25.0 5.0 2>/dev/null || true
sleep 0.2
"${ADB}" -s "${SENDER}" emu sensor set acceleration 0 9.8 0.8 2>/dev/null || true
"${ADB}" -s "${RECEIVER}" emu sensor set acceleration 0 9.8 0.8 2>/dev/null || true

# 5. Allow Autonomous NFC Background Polling & Handshake
echo "[5/6] Waiting for background reader mode & HCE exchange..."
for i in {1..8}; do
    echo "  Holding proximity (contact time: ${i}s)..."
    sleep 1
done

echo ""
echo "=========================================================="
echo " Scenario 1 Logcat Analysis (Sender):"
echo "=========================================================="
"${ADB}" -s "${SENDER}" logcat -d -s GestureExchange:V NearbySharing:V NfcService:D HostEmulationManager:V | tail -n 35 || true

echo ""
echo "=========================================================="
echo " Scenario 1 Logcat Analysis (Receiver):"
echo "=========================================================="
"${ADB}" -s "${RECEIVER}" logcat -d -s GestureExchange:V NearbySharing:V NfcService:D HostEmulationManager:V | tail -n 35 || true

echo ""
echo "=========================================================="
echo " Checking Foreground Activities on Both Devices:"
echo "=========================================================="
echo "--- Sender Foreground Activity ---"
"${ADB}" -s "${SENDER}" shell dumpsys window | grep -E "mCurrentFocus|mFocusedApp" || true
echo "--- Receiver Foreground Activity ---"
"${ADB}" -s "${RECEIVER}" shell dumpsys window | grep -E "mCurrentFocus|mFocusedApp" || true

echo ""
echo "=========================================================="
echo " Final Radio Statistics:"
echo "=========================================================="
"${NETSIM_BIN}" devices

echo "=========================================================="
echo " Scenario 1 Verification Evaluation:"
echo "=========================================================="
SENDER_LOG=$("${ADB}" -s "${SENDER}" logcat -d -s GestureExchange:V NearbySharing:V HostEmulationManager:V 2>/dev/null || true)
RECEIVER_LOG=$("${ADB}" -s "${RECEIVER}" logcat -d -s GestureExchange:V NearbySharing:V HostEmulationManager:V 2>/dev/null || true)

SENDER_EVENTS=$(echo "${SENDER_LOG}" | grep -iE "A00000047609|transceive|SELECT_PRIMARY_AID" | wc -l)
RECEIVER_EVENTS=$(echo "${RECEIVER_LOG}" | grep -iE "A00000047609|HostApdu|handleSelectAid" | wc -l)

if [ "${SENDER_EVENTS}" -gt 0 ] && [ "${RECEIVER_EVENTS}" -gt 0 ]; then
    echo " [RESULT: PASS] Tap-to-X Scenario 1: Contacts Exchange Passed (${SENDER_EVENTS} sender / ${RECEIVER_EVENTS} receiver events)"
    echo "=========================================================="
else
    echo " [RESULT: FAIL] Tap-to-X Scenario 1: No bilateral NFC APDU exchange detected (Sender: ${SENDER_EVENTS}, Receiver: ${RECEIVER_EVENTS})"
    echo "=========================================================="
    exit 1
fi

