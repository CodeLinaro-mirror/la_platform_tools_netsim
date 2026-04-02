/*
 * Copyright 2026 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */
package com.android.netsim.agent

import android.content.Context
import android.util.Log
import kotlin.math.abs
import kotlin.random.Random
import kotlinx.coroutines.TimeoutCancellationException
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout

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
  Log.i(TAG, "Disabling UWB via settings and stopping active sessions")
  try {
    // 1. Update system setting
    val p = Runtime.getRuntime().exec("settings put global uwb_enabled 0")
    p.waitFor()

    // 2. Explicitly stop the application ranging session
    UwbSessionManager.stopRanging()
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

/// STEP: ^configures UWB session ID (.*)$
fun configureUwbSessionId(context: Context, args: List<String>) {
  val sessionId = args[0].toUInt()
  Log.i(TAG, "Configuring UWB Session ID: $sessionId")
  UwbSessionManager.currentSessionId = sessionId
}

/// STEP: ^starts UWB ranging with session (.*) and config (\d+) and peer (.*)$
fun startUwbRangingSessionConfig(context: Context, args: List<String>) {
  val sessionId = args[0].toUInt()
  val configId = args[1].toInt()
  val peer = args[2]

  UwbSessionManager.currentSessionId = sessionId
  UwbSessionManager.startRanging(listOf(peer), configId)
}

/// STEP: ^starts UWB ranging with config (\d+) and peer (.*)$
fun startUwbRangingRoleConfig(context: Context, args: List<String>) {
  val configId = args[0].toInt()
  val peer = args[1]

  UwbSessionManager.startRanging(listOf(peer), configId)
}

/// STEP: ^Starts UWB Ranging with (.*)$
fun startUwbRangingSimple(context: Context, args: List<String>) {
  UwbSessionManager.startRanging(listOf(args[0]), 1)
}

/// STEP: ^Stops UWB Ranging$
fun stopUwbRanging(context: Context, args: List<String>) {
  UwbSessionManager.stopRanging()
}

/// STEP: ^starts UWB ranging with session (.*) and peer (.*)$
fun startUwbRangingSessionPeer(context: Context, args: List<String>) {
  val sessionId = args[0].toUInt()
  val peer = args[1]
  val configId = 1

  UwbSessionManager.currentSessionId = sessionId
  UwbSessionManager.startRanging(listOf(peer), configId)
}

/// STEP: ^starts UWB ranging with peer (.*)$
fun startUwbRangingPeerRole(context: Context, args: List<String>) {
  val peer = args[0]
  val configId = 1

  UwbSessionManager.startRanging(listOf(peer), configId)
}

/// STEP: ^starts UWB ranging with peers (.*) as (CONTROLLER|CONTROLEE)$
fun startUwbRangingMultiPeer(context: Context, args: List<String>) {
  val peers = args[0].split(",").map { it.trim() }
  val role = args[1].uppercase()
  val configId = 1
  UwbSessionManager.startRanging(peers, configId)
}

/// STEP: ^starts UWB ranging with config (\d+) and peers (.*)$
fun startUwbRangingMultiPeerWithConfig(context: Context, args: List<String>) {
  val configId = args[0].toInt()
  val peers = args[1].split(",").map { it.trim() }
  UwbSessionManager.startRanging(peers, configId)
}

/// STEP: ^UWB distance to (.*) is ([\d\.]+)m \(\+/- ([\d\.]+)m\)$
fun verifyUwbDistance(context: Context, args: List<String>) {
  val to = args[0]
  val expectedDist = args[1].toFloat()
  val tolerance = args[2].toFloat()

  try {
    runBlocking {
      withTimeout(5000L) {
        UwbSessionManager.lastDistanceMap.first { map ->
          val lastVal = map[to]
          lastVal != null && abs(lastVal - expectedDist) <= tolerance
        }
      }
    }
  } catch (e: TimeoutCancellationException) {
    val currentValues = UwbSessionManager.lastDistanceMap.value
    throw Exception(
      "UWB distance mismatch to $to. Expected: $expectedDist +/- $tolerance. Current values: $currentValues"
    )
  }
}

/// STEP: ^UWB azimuth to (.*) is ([\d\.]+) degrees \(\+/- ([\d\.]+) degrees\)$
fun verifyUwbAzimuth(context: Context, args: List<String>) {
  val to = args[0]
  val expectedAz = args[1].toFloat()
  val tolerance = args[2].toFloat()

  try {
    runBlocking {
      withTimeout(5000L) {
        UwbSessionManager.lastAzimuthMap.first { map ->
          val lastVal = map[to]
          lastVal != null && abs(lastVal - expectedAz) <= tolerance
        }
      }
    }
  } catch (e: TimeoutCancellationException) {
    val currentValues = UwbSessionManager.lastAzimuthMap.value
    throw Exception(
      "UWB azimuth mismatch to $to. Expected: $expectedAz +/- $tolerance. Current values: $currentValues"
    )
  }
}

/// STEP: ^UWB elevation to (.*) is ([\d\.]+) degrees \(\+/- ([\d\.]+) degrees\)$
fun verifyUwbElevation(context: Context, args: List<String>) {
  val to = args[0]
  val expectedEl = args[1].toFloat()
  val tolerance = args[2].toFloat()

  try {
    runBlocking {
      withTimeout(5000L) {
        UwbSessionManager.lastElevationMap.first { map ->
          val lastVal = map[to]
          lastVal != null && abs(lastVal - expectedEl) <= tolerance
        }
      }
    }
  } catch (e: TimeoutCancellationException) {
    val currentValues = UwbSessionManager.lastElevationMap.value
    throw Exception(
      "UWB elevation mismatch to $to. Expected: $expectedEl +/- $tolerance. Current values: $currentValues"
    )
  }
}

/// STEP: ^UWB (distance|azimuth|elevation) to (.*) is not available$
fun verifyUwbMeasurementNotAvailable(context: Context, args: List<String>) {
  val type = args[0]
  val to = args[1]
  // Wait a bit to be sure it's not coming
  Thread.sleep(2000)
  val available =
    when (type) {
      "distance" -> UwbSessionManager.lastDistanceMap.value.containsKey(to)
      "azimuth" -> UwbSessionManager.lastAzimuthMap.value.containsKey(to)
      "elevation" -> UwbSessionManager.lastElevationMap.value.containsKey(to)
      else -> false
    }
  if (available) {
    throw Exception("UWB $type to $to is available but expected not to be")
  }
}

/// STEP: ^UWB session state is (.*)$
fun verifyUwbSessionState(context: Context, args: List<String>) {
  val expected = args[0]
  try {
    runBlocking {
      withTimeout(5000L) {
        UwbSessionManager.sessionState.first { it.equals(expected, ignoreCase = true) }
      }
    }
  } catch (e: TimeoutCancellationException) {
    val lastState = UwbSessionManager.sessionState.value
    throw Exception("Timeout waiting for UWB session state $expected. Actual: $lastState")
  }
}

/// STEP: ^UWB is (connected|disconnected)$
fun verifyUwbPeerStateGlobal(context: Context, args: List<String>) {
  val expectedState = if (args[0] == "connected") "Ranging" else "Disconnected"

  try {
    runBlocking {
      withTimeout(5000L) {
        UwbSessionManager.sessionState.first { state ->
          if (expectedState == "Ranging") {
            state == "Ranging"
          } else {
            state == "Disconnected" || state == "Stopped" || state == "Idle"
          }
        }
      }
    }
  } catch (e: TimeoutCancellationException) {
    val lastState = UwbSessionManager.sessionState.value
    if (expectedState == "Ranging") {
      throw Exception("Timeout waiting for UWB Peer connected (Ranging state). State: $lastState")
    } else {
      throw Exception("Timeout waiting for UWB Peer disconnected. State: $lastState")
    }
  }
}

/// STEP: ^UWB peer (.*) is (connected|disconnected)$
fun verifyUwbPeerStateSpecific(context: Context, args: List<String>) {
  val peer = args[0]
  val expectedStatus = if (args[1] == "connected") "Connected" else "Disconnected"

  try {
    runBlocking {
      withTimeout(5000L) {
        UwbSessionManager.peerStatusMap.first {
          val status = it[peer]
          status == expectedStatus || (expectedStatus == "Disconnected" && status == null)
        }
      }
    }
  } catch (e: TimeoutCancellationException) {
    val currentStatuses = UwbSessionManager.peerStatusMap.value
    throw Exception(
      "Timeout waiting for UWB peer $peer to be $expectedStatus. Current statuses: $currentStatuses"
    )
  }
}

/// STEP: ^initializes UWB (CONTROLLER|CONTROLEE) session$
fun initUwbSessionWithRole(context: Context, args: List<String>) {
  val role = args[0]
  val isController = role.equals("CONTROLLER", ignoreCase = true)
  UwbSessionManager.initSession(context, isController)
}

/// STEP: ^is a UWB (CONTROLLER|CONTROLEE) with session (.*) and address (.*)$
fun setupUwbDevice(context: Context, args: List<String>): Any {
  val role = args[0]
  val sessionArg = args[1]
  val addrVarName = args[2]
  return setupUwbDeviceInternal(context, role, sessionArg, addrVarName)
}

/// STEP: ^is a UWB (CONTROLLER|CONTROLEE) with address (.*)$
fun setupUwbDeviceNoSession(context: Context, args: List<String>): Any {
  val role = args[0]
  val addrVarName = args[1]
  return setupUwbDeviceInternal(context, role, null, addrVarName)
}

fun setupUwbDeviceInternal(
  context: Context,
  role: String,
  sessionArg: String?,
  addrVarName: String,
): Any {

  var sessionId: UInt
  val returns = mutableMapOf<String, Any>()

  if (sessionArg == null) {
    sessionId = UwbSessionManager.DEFAULT_SESSION_ID
  } else if (sessionArg.toUIntOrNull() != null) {
    sessionId = sessionArg.toUInt()
  } else {
    sessionId = Random.nextInt().toUInt()
    returns[sessionArg] = sessionId
  }

  Log.i(TAG, "Setting up UWB as $role with Session ID: $sessionId")

  // 1. Configure default channel/preamble and requested session ID
  UwbSessionManager.currentChannel = 9
  UwbSessionManager.currentPreambleIndex = 11
  UwbSessionManager.currentSessionId = sessionId

  // 2. Enable UWB
  Runtime.getRuntime().exec("settings put global uwb_enabled 1").waitFor()
  Thread.sleep(1000) // Give the radio a moment to turn on

  // 3. Initialize Session
  val isController = role.equals("CONTROLLER", ignoreCase = true)
  UwbSessionManager.initSession(context, isController)

  // 4. Wait for and return the local address
  val addr =
    try {
      runBlocking { withTimeout(5000L) { UwbSessionManager.localAddress.first { it != null } } }
    } catch (e: TimeoutCancellationException) {
      throw Exception("Timeout waiting for UWB local address during setup")
    }

  Log.i(TAG, "UWB Setup Complete. Address: $addr saved to $addrVarName")
  returns[addrVarName] = addr!!
  return returns
}

/// STEP: ^UWB address is available$
fun waitForUwbAddress(context: Context, args: List<String>): Any {
  val addr =
    try {
      runBlocking { withTimeout(5000L) { UwbSessionManager.localAddress.first { it != null } } }
    } catch (e: TimeoutCancellationException) {
      throw Exception("Timeout waiting for UWB local address")
    }

  Log.i(TAG, "UWB Address Available: $addr")
  return mapOf("UWB_ADDRESS" to addr!!)
}

/// STEP: ^UWB address is available as (.*)$
fun waitForUwbAddressAsVar(context: Context, args: List<String>): Any {
  val addr =
    try {
      runBlocking { withTimeout(5000L) { UwbSessionManager.localAddress.first { it != null } } }
    } catch (e: TimeoutCancellationException) {
      throw Exception("Timeout waiting for UWB local address")
    }

  val varName = args[0]
  Log.i(TAG, "UWB Address Available: $addr saved to $varName")
  return mapOf(varName to addr!!)
}
