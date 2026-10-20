/*
 * Copyright 2026 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */

package com.android.verify.core

interface FeatureObservable {
  fun getObservables(): Map<String, String>
}
