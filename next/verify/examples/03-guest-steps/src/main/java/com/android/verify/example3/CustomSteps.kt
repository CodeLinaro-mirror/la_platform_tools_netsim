/*
 * Copyright 2026 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */
package com.android.verify.example3

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import com.android.verify.core.Step

/** Custom steps for Example 3. */
@Step("Android says hello to \"([^\"]+)\"")
fun sayHello(context: Context, args: List<String>) {
  val name = args[0]
  android.util.Log.i("CustomSteps", "Hello, $name!")
}

@Step("Android checks wifi is connected")
fun checkWifiConnected(context: Context, args: List<String>) {
  val cm = context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
  val activeNetwork = cm.activeNetwork
  val capabilities = cm.getNetworkCapabilities(activeNetwork)
  val isConnected = capabilities?.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) == true
  check(isConnected) { "Wifi is not connected!" }
  android.util.Log.i("CustomSteps", "Wifi is connected!")
}
