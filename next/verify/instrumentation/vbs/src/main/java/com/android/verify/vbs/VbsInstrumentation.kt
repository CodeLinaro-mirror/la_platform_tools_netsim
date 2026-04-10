/*
 * Copyright 2026 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */

package com.android.verify.vbs

import android.os.Bundle
import android.util.Log
import com.android.verify.core.VerifyInstrumentation

class VbsInstrumentation : VerifyInstrumentation() {
  private val TAG = "VbsInstrumentation"

  override fun onCreate(arguments: Bundle) {
    Log.i(TAG, "VbsInstrumentation.onCreate starting")
    super.onCreate(arguments)

    val context = getContext()
    BluetoothState.reset(context)

    // Connect BluetoothState logger to our log function
    BluetoothState.logger = { msg ->
      if (msg.startsWith("SCANNED")) {
        log("${Protocol.INFO_TAG} $msg")
      } else {
        Log.i(TAG, msg)
      }
    }
  }

  override fun registerSteps() {
    val r = registry ?: return
    val context = getContext()

    WifiStepsLoader.loadSteps(r, context)
    ConnectivityStepsLoader.loadSteps(r, context)
    NetworkStepsLoader.loadSteps(r, context)
    LifecycleStepsLoader.loadSteps(r, context)
    ServiceDiscoveryLoader.loadSteps(r, context)
    TcpEchoStepsLoader.loadSteps(r, context)
    UwbStepsLoader.loadSteps(r, context)
    BluetoothAdvStepsLoader.loadSteps(r, context)
  }
}
