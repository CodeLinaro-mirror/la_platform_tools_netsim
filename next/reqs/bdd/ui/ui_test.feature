# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

@ui
Feature: UI Automator Support Test

  Scenario: Toggle Wi-Fi via Settings UI
    Given @adb has 1 attached device
    When @android:1 sets Wi-Fi to disabled via UI
    Then @android:1 Android Wi-Fi is disabled
    When @android:1 sets Wi-Fi to enabled via UI
    Then @android:1 Android Wi-Fi is enabled
