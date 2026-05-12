# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

Feature: Custom Host Steps Example

  Scenario: Host says hello
    When Host says hello to "World"

  Scenario: Host checks devices
    Given Host checks for devices
