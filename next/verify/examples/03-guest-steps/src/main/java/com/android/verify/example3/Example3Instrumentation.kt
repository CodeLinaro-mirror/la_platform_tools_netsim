/*
 * Copyright 2026 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */
package com.android.verify.example3

import android.os.Bundle
import android.util.Log
import com.android.verify.core.VerifyInstrumentation

/**
 * Example instrumentation for custom guest steps.
 *
 * This example explicitly registers step classes instead of relying on fragile runtime reflection
 * scanning. This is the recommended approach for custom APKs.
 */
class Example3Instrumentation : VerifyInstrumentation() {
  private val TAG = "Example3Instrumentation"

  override fun onCreate(arguments: Bundle) {
    super.onCreate(arguments)
    Log.i(TAG, "Example3Instrumentation.onCreate starting")
  }

  override fun registerSteps() {
    val r = registry ?: return

    // Explicitly register the class containing custom steps.
    // This avoids fragile reflection scanning of the dex file.
    // We use Class.forName because Kotlin top-level functions are compiled into a class
    // named after the file with "Kt" suffix, which is not directly accessible as a type in Kotlin.
    try {
      val clazz = Class.forName("com.android.verify.example3.CustomStepsKt")
      r.registerStepsFromClass(clazz)
    } catch (e: ClassNotFoundException) {
      Log.e(TAG, "Failed to find CustomStepsKt class", e)
    }
  }
}
