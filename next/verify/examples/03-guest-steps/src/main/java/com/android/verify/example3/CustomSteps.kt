/*
 * Copyright 2026 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */
package com.android.verify.example3

import android.content.Context
import com.android.verify.core.Step

/**
 * Custom steps for Example 3.
 *
 * NOTE: We are only showing a trivial Log.i step here because privileged steps (like checking WiFi
 * state) failed to be discovered by the reflection scanner in this standalone APK setup. See
 * b/508641214 for tracking the enhancement to use standard steps and resolve discovery issues.
 */
@Step("Android says hello to \"([^\"]+)\"")
fun sayHello(context: Context, args: List<String>) {
  val name = args[0]
  android.util.Log.i("CustomSteps", "Hello, $name!")
}
