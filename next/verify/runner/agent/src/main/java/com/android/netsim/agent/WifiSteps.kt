/*
 * Copyright 2026 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */
package com.android.netsim.agent

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.location.LocationManager
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.net.wifi.WifiManager
import android.util.Log

private const val WIFI_TAG = "Verify:WiFi"

/// STEP: ^When Android Connects to Wifi$
fun connectToWifi(context: Context, args: List<String>) {
  Log.i(WIFI_TAG, "Ensuring Wifi is enabled")
  var success = false
  try {
    val p1 = Runtime.getRuntime().exec("svc wifi enable")
    if (p1.waitFor() == 0) success = true

    val p2 = Runtime.getRuntime().exec("cmd wifi set-wifi-enabled enabled")
    if (p2.waitFor() == 0) success = true

    if (success) {
      Thread.sleep(3000)
    }
  } catch (e: Exception) {
    throw Exception("Exception enabling wifi: ${e.message}")
  }

  if (!success) {
    throw Exception("Failed to enable Wifi via shell commands (exit code non-zero)")
  }
}

/// STEP: ^Then Wi-Fi device info shows SSID "(.*)"$
fun verifyWifiInfoSsid(context: Context, args: List<String>) {
  val expectedSsid = args[0]
  Log.i(WIFI_TAG, "Verifying Wi-Fi device info SSID is: $expectedSsid")

  val lm = context.getSystemService(Context.LOCATION_SERVICE) as LocationManager
  if (!lm.isLocationEnabled) {
    throw Exception("Location services are disabled on the device! Cannot get SSID.")
  }

  val fineGranted =
    context.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) ==
      PackageManager.PERMISSION_GRANTED
  val bgGranted =
    context.checkSelfPermission(Manifest.permission.ACCESS_BACKGROUND_LOCATION) ==
      PackageManager.PERMISSION_GRANTED

  Log.i(WIFI_TAG, "App has ACCESS_FINE_LOCATION: $fineGranted")
  Log.i(WIFI_TAG, "App has ACCESS_BACKGROUND_LOCATION: $bgGranted")

  if (!fineGranted) {
    throw Exception("ACCESS_FINE_LOCATION permission not granted! Cannot get SSID.")
  }
  if (!bgGranted) {
    throw Exception("ACCESS_BACKGROUND_LOCATION permission not granted! Cannot get SSID.")
  }

  val cm = context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
  var ssid: String? = null
  val startTime = System.currentTimeMillis()
  val timeoutMs = 60000 // 60 seconds

  Log.i(WIFI_TAG, "Polling for Wi-Fi capabilities for expected SSID $expectedSsid...")
  while (System.currentTimeMillis() - startTime < timeoutMs) {
    val activeNetwork = cm.activeNetwork
    if (activeNetwork != null) {
      val capabilities = cm.getNetworkCapabilities(activeNetwork)
      if (capabilities != null && capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)) {
        val wifiInfo = capabilities.transportInfo as? android.net.wifi.WifiInfo
        ssid = wifiInfo?.ssid?.trim('"')
        Log.i(WIFI_TAG, "Polled SSID: $ssid")
        if (ssid == expectedSsid) {
          break
        }
      }
    }
    Thread.sleep(2000) // Poll every 2 seconds
  }

  if (ssid != expectedSsid) {
    throw Exception("Expected SSID $expectedSsid from NetworkCapabilities, but got $ssid")
  }
}

/// STEP: ^Then Android is connected to Wi-Fi SSID "(.*)"$
fun verifyAndroidIsConnectedToWifi(context: Context, args: List<String>) {
  val expectedSsid = args[0]
  Log.i(WIFI_TAG, "Verifying Android is connected to Wi-Fi SSID: $expectedSsid")
  // Re-use verifyWifiInfoSsid implementation
  verifyWifiInfoSsid(context, args)
}

/// STEP: ^When Android releases Wi-Fi connection$
fun releaseWifiConnection(context: Context, args: List<String>) {
  Log.i(WIFI_TAG, "Releasing Wi-Fi connection")
  val wm = context.getSystemService(Context.WIFI_SERVICE) as WifiManager
  wm.disconnect()
}
