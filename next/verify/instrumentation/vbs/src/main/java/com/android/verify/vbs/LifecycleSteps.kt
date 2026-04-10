/*
 * Copyright 2026 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */
package com.android.verify.vbs

import android.content.Context
import com.android.verify.core.QuitException

/// STEP: ^resets world$
fun resetState(context: Context, args: List<String>) {
  BluetoothState.reset(context)
  UwbSessionManager.reset(context)
  DiscoveryState.reset(context)
}

/// STEP: ^THEN Android Quits$
fun quit(context: Context, args: List<String>) {
  DiscoveryState.reset(context)
  throw QuitException()
}
