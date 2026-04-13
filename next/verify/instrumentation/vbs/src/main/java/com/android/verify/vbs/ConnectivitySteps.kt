/*
 * Copyright 2026 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */
package com.android.verify.vbs

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.net.wifi.WifiManager
import android.util.Log
import com.android.verify.core.Step

private const val TAG = "Verify:Connectivity"

@Step("When Android connects to Wi-Fi SSID \"(.*)\"")
fun connectToWifiSsid(context: Context, args: List<String>) {
  connectToWifiInternal(context, args[0], null)
}

@Step("When Android connects to Wi-Fi SSID \"([^\"]+)\" with password \"([^\"]+)\"")
fun connectToSecuredWifiSsid(context: Context, args: List<String>) {
  connectToWifiInternal(context, args[0], args[1])
}

private fun connectToWifiInternal(context: Context, ssid: String, password: String?) {
  Log.i(TAG, "Connecting to Wi-Fi SSID: $ssid" + if (password != null) " with password" else "")

  val cm = context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
  val wm = context.getSystemService(Context.WIFI_SERVICE) as WifiManager

  // TODO: Move to modern WifiNetworkSpecifier/WifiNetworkSuggestions when UI Automator
  // is available to handle modern interactive prompt dialogs.
  @Suppress("DEPRECATION")
  val wifiConfig =
    android.net.wifi.WifiConfiguration().apply {
      SSID = "\"$ssid\""
      if (password != null) {
        preSharedKey = "\"$password\""
      } else {
        allowedKeyManagement.set(android.net.wifi.WifiConfiguration.KeyMgmt.NONE)
      }
    }

  @Suppress("DEPRECATION") val netId = wm.addNetwork(wifiConfig)
  if (netId == -1) {
    throw Exception("Failed to add network configuration for $ssid")
  }

  @Suppress("DEPRECATION") wm.disconnect()
  @Suppress("DEPRECATION") wm.enableNetwork(netId, true)
  @Suppress("DEPRECATION") wm.reconnect()

  Log.i(TAG, "Legacy addNetwork called for ${if (password != null) "secured " else ""}$ssid")

  val request =
    NetworkRequest.Builder().addTransportType(NetworkCapabilities.TRANSPORT_WIFI).build()

  val latch = java.util.concurrent.CountDownLatch(1)
  val callback =
    object : ConnectivityManager.NetworkCallback(1) { // FLAG_INCLUDE_LOCATION_INFO
      override fun onCapabilitiesChanged(network: Network, capabilities: NetworkCapabilities) {
        val wifiInfo = capabilities.transportInfo as? android.net.wifi.WifiInfo
        val currentSsid = wifiInfo?.ssid?.trim('"')
        Log.i(TAG, "onCapabilitiesChanged: SSID=$currentSsid")
        if (currentSsid == ssid) {
          latch.countDown()
        }
      }
    }

  cm.registerNetworkCallback(request, callback)

  Log.i(
    TAG,
    "Waiting for Wi-Fi to auto-connect to suggested ${if (password != null) "secured " else ""}network $ssid...",
  )
  if (!latch.await(90, java.util.concurrent.TimeUnit.SECONDS)) {
    cm.unregisterNetworkCallback(callback)
    throw Exception(
      "Timeout waiting for Wi-Fi to auto-connect to suggested ${if (password != null) "secured " else ""}network $ssid"
    )
  }
  cm.unregisterNetworkCallback(callback)
}
