/*
 * Copyright 2026 The Android Open Source Project
 */
package com.android.netsim.agent

import android.content.Context
import android.util.Log
import kotlin.math.abs

private const val TAG = "UwbSteps"

/// STEP: ^enables UWB$
fun enableUwb(context: Context, args: List<String>) {
  Log.i(TAG, "Enabling UWB via settings")
  try {
    val p = Runtime.getRuntime().exec("settings put global uwb_enabled 1")
    if (p.waitFor() != 0) {
      Log.w(TAG, "Failed to enable UWB via settings (exit code non-zero)")
    }
    Thread.sleep(1000)
  } catch (e: Exception) {
    throw Exception("Failed to enable UWB: ${e.message}")
  }
}

/// STEP: ^disables UWB$
fun disableUwb(context: Context, args: List<String>) {
  Log.i(TAG, "Disabling UWB via settings")
  try {
    val p = Runtime.getRuntime().exec("settings put global uwb_enabled 0")
    p.waitFor()
  } catch (e: Exception) {
    throw Exception("Failed to disable UWB: ${e.message}")
  }
}

/// STEP: ^configures UWB channel (\d+) and preamble (\d+)$
fun configureUwbChannelPreamble(context: Context, args: List<String>) {
  val channel = args[0].toInt()
  val preamble = args[1].toInt()
  Log.i(TAG, "Configuring UWB Channel: $channel, Preamble: $preamble")
  UwbSessionManager.currentChannel = channel
  UwbSessionManager.currentPreambleIndex = preamble
}

/// STEP: ^configures UWB session ID (\d+)$
fun configureUwbSessionId(context: Context, args: List<String>) {
  val sessionId = args[0].toInt()
  Log.i(TAG, "Configuring UWB Session ID: $sessionId")
  UwbSessionManager.currentSessionId = sessionId
}

/// STEP: ^starts UWB ranging as (Controller|Controlee) with config (\d+) and peer (.*)$
fun startUwbRangingRoleConfig(context: Context, args: List<String>) {
  val role = args[0]
  val configId = args[1].toInt()
  val peer = args[2]

  if (role == "Controller") {
    UwbSessionManager.startRanging(peer, configId)
  } else {
    UwbSessionManager.startRanging(peer, configId)
  }
}

/// STEP: ^Starts UWB Ranging with (.*)$
fun startUwbRangingSimple(context: Context, args: List<String>) {
  UwbSessionManager.startRanging(args[0], 1)
}

/// STEP: ^Stops UWB Ranging$
fun stopUwbRanging(context: Context, args: List<String>) {
  UwbSessionManager.stopRanging()
}

/// STEP: ^starts UWB ranging with peer (.*) as (CONTROLLER|CONTROLEE|Controller|Controlee)$
fun startUwbRangingPeerRole(context: Context, args: List<String>) {
  val peer = args[0]
  val role = args[1].uppercase()
  val configId = 1

  if (role == "CONTROLLER") {
    UwbSessionManager.startRanging(peer, configId)
  } else {
    UwbSessionManager.startRanging(peer, configId)
  }
}

/// STEP: ^UWB distance is ([\d\.]+) meters with tolerance ([\d\.]+) meters$
fun verifyUwbDistance(context: Context, args: List<String>) {
  val expectedDist = args[0].toFloat()
  val tolerance = args[1].toFloat()

  var retries = 50
  var lastVal: Float? = null
  while (retries > 0) {
    lastVal = UwbSessionManager.lastDistance.value
    if (lastVal != null && abs(lastVal - expectedDist) <= tolerance) {
      return
    }
    Thread.sleep(100)
    retries--
  }

  if (lastVal == null) {
    throw Exception("No UWB distance measurement available after timeout")
  }

  if (abs(lastVal - expectedDist) > tolerance) {
    throw Exception(
      "UWB distance mismatch. Expected: $expectedDist +/- $tolerance, Actual: $lastVal"
    )
  }
}

/// STEP: ^UWB azimuth is ([\d\.]+) degrees with tolerance ([\d\.]+) degrees$
fun verifyUwbAzimuth(context: Context, args: List<String>) {
  val expectedAz = args[0].toFloat()
  val tolerance = args[1].toFloat()

  var retries = 50
  var lastVal: Float? = null
  while (retries > 0) {
    lastVal = UwbSessionManager.lastAzimuth.value
    if (lastVal != null && abs(lastVal - expectedAz) <= tolerance) {
      return
    }
    Thread.sleep(100)
    retries--
  }

  if (lastVal == null) {
    throw Exception("No UWB azimuth measurement available after timeout")
  }

  if (abs(lastVal - expectedAz) > tolerance) {
    throw Exception("UWB azimuth mismatch. Expected: $expectedAz +/- $tolerance, Actual: $lastVal")
  }
}

/// STEP: ^UWB elevation is ([\d\.]+) degrees with tolerance ([\d\.]+) degrees$
fun verifyUwbElevation(context: Context, args: List<String>) {
  val expectedEl = args[0].toFloat()
  val tolerance = args[1].toFloat()

  var retries = 50
  var lastVal: Float? = null
  while (retries > 0) {
    lastVal = UwbSessionManager.lastElevation.value
    if (lastVal != null && abs(lastVal - expectedEl) <= tolerance) {
      return
    }
    Thread.sleep(100)
    retries--
  }

  if (lastVal == null) {
    throw Exception("No UWB elevation measurement available after timeout")
  }

  if (abs(lastVal - expectedEl) > tolerance) {
    throw Exception(
      "UWB elevation mismatch. Expected: $expectedEl +/- $tolerance, Actual: $lastVal"
    )
  }
}

/// STEP: ^UWB peer (.*) is (connected|disconnected)$
fun verifyUwbPeerState(context: Context, args: List<String>) {
  val expectedState = if (args[1] == "connected") "Ranging" else "Disconnected"

  var retries = 50
  var lastState = ""
  while (retries > 0) {
    lastState = UwbSessionManager.sessionState.value

    if (expectedState == "Ranging" && lastState == "Ranging") {
      return
    }
    if (
      expectedState == "Disconnected" &&
        (lastState == "Disconnected" || lastState == "Stopped" || lastState == "Idle")
    ) {
      return
    }
    Thread.sleep(100)
    retries--
  }

  if (expectedState == "Ranging" && lastState != "Ranging") {
    throw Exception("Timeout waiting for UWB Peer connected. State: $lastState")
  }
  if (
    expectedState == "Disconnected" &&
      lastState != "Disconnected" &&
      lastState != "Stopped" &&
      lastState != "Idle"
  ) {
    throw Exception("Timeout waiting for UWB Peer disconnected. State: $lastState")
  }
}

/// STEP: ^initializes UWB (CONTROLLER|CONTROLEE) session$
fun initUwbSessionWithRole(context: Context, args: List<String>) {
  val role = args[0]
  val isController = role.equals("CONTROLLER", ignoreCase = true)
  UwbSessionManager.initSession(context, isController)
}

/// STEP: ^UWB address is available$
fun waitForUwbAddress(context: Context, args: List<String>): Any {
  var retries = 50
  while (UwbSessionManager.localAddress.value == null && retries > 0) {
    Thread.sleep(100)
    retries--
  }

  val addr = UwbSessionManager.localAddress.value
  if (addr == null) {
    throw Exception("Timeout waiting for UWB local address")
  }
  Log.i(TAG, "UWB Address Available: $addr")
  return mapOf("UWB_ADDRESS" to addr)
}

/// STEP: ^UWB address is available as (.*)$
fun waitForUwbAddressAsVar(context: Context, args: List<String>): Any {
  var retries = 50
  while (UwbSessionManager.localAddress.value == null && retries > 0) {
    Thread.sleep(100)
    retries--
  }

  val addr = UwbSessionManager.localAddress.value
  if (addr == null) {
    throw Exception("Timeout waiting for UWB local address")
  }
  val varName = args[0]
  Log.i(TAG, "UWB Address Available: $addr saved to $varName")
  return mapOf(varName to addr)
}
