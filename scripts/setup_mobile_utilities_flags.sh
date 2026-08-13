#!/bin/bash
# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0
#
# Setup network and Mobile Utilities app on connected Android emulators.

set -euo pipefail

SDK_DIR="${ANDROID_SDK_ROOT:-${HOME}/Library/Android/sdk}"
ADB_BIN="${SDK_DIR}/platform-tools/adb"
MOBUTILS_APK="${HOME}/Downloads/mobileutilities_binary.apk"

DEVICES=("${@:-emulator-5554 emulator-5556}")

echo "=========================================================="
echo " Setting Up Network & Mobile Utilities App"
echo " Target Devices: ${DEVICES[*]}"
echo "=========================================================="

for DEV in "${DEVICES[@]}"; do
    echo "--- Configuring $DEV ---"

    # 1. Wake & unlock
    "${ADB_BIN}" -s "$DEV" shell input keyevent KEYCODE_WAKEUP 2>/dev/null || true
    "${ADB_BIN}" -s "$DEV" shell wm dismiss-keyguard 2>/dev/null || true

    # 2. Fix Wi-Fi & Private DNS for SLIRP NAT
    echo "  [1/2] Enabling Wi-Fi and setting private DNS off..."
    "${ADB_BIN}" -s "$DEV" shell svc wifi enable 2>/dev/null || true
    "${ADB_BIN}" -s "$DEV" shell settings put global private_dns_mode off 2>/dev/null || true

    # 3. Install Mobile Utilities APK if available
    if [ -f "$MOBUTILS_APK" ]; then
        echo "  [2/2] Installing Mobile Utilities from $MOBUTILS_APK..."
        "${ADB_BIN}" -s "$DEV" install -r -g "$MOBUTILS_APK" 2>/dev/null || true
    fi
done

echo "=========================================================="
echo " Network & Mobile Utilities setup complete!"
echo "=========================================================="
