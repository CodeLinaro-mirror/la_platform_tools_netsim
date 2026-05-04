# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

Feature: Custom Guest Steps Example

  Scenario: Android says hello
    Given @avd has 1 attached device
    When @avd:1 says hello to "World"
