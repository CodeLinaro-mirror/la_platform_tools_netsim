/*
 * Copyright 2026 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */
package com.android.netsim.agent

import android.content.Context
import android.util.Log
import androidx.core.uwb.RangingParameters
import androidx.core.uwb.RangingResult
import androidx.core.uwb.UwbAddress
import androidx.core.uwb.UwbComplexChannel
import androidx.core.uwb.UwbDevice
import androidx.core.uwb.UwbManager
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.cancel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

@OptIn(ExperimentalStdlibApi::class)
object UwbSessionManager {
  private const val TAG = "UwbSessionManager"
  private var rangingJob: Job? = null
  private val scope = CoroutineScope(Dispatchers.IO)

  // Last observed results for verification steps, keyed by peerAddress hex string
  val lastDistanceMap = MutableStateFlow<Map<String, Float>>(emptyMap())
  val lastAzimuthMap = MutableStateFlow<Map<String, Float>>(emptyMap())
  val lastElevationMap = MutableStateFlow<Map<String, Float>>(emptyMap())

  // Peer status tracking, keyed by peer hex address
  val peerStatusMap = MutableStateFlow<Map<String, String>>(emptyMap())

  // Session state: "Idle", "Initializing", "Initialized", "Active", "Ranging", "Stopped",
  // "Disconnected"
  val sessionState = MutableStateFlow<String>("Idle")

  const val DEFAULT_SESSION_ID: UInt = 12345678u

  // Default parameters (can be overridden by steps)
  var currentChannel = 9
  var currentPreambleIndex = 9
  var currentSessionId: UInt = DEFAULT_SESSION_ID
  var currentSessionKey = byteArrayOf(0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08)

  // Stored scopes for split initialization
  private var controllerSession: androidx.core.uwb.UwbControllerSessionScope? = null
  private var controleeSession: androidx.core.uwb.UwbControleeSessionScope? = null

  // Exposed local address for OOB simulation
  val localAddress = MutableStateFlow<String?>(null)

  fun initSession(context: Context, isController: Boolean) {
    if (
      (isController && controllerSession != null) || (!isController && controleeSession != null)
    ) {
      Log.i(TAG, "Session already initialized")
      return
    }
    stopRanging()
    sessionState.value = "Initializing"
    localAddress.value = null
    controllerSession = null
    controleeSession = null

    val uwbManager = UwbManager.createInstance(context)

    rangingJob = scope.launch {
      try {
        if (isController) {
          val s = uwbManager.controllerSessionScope()
          controllerSession = s
          val addr = s.localAddress.address.toHexString()
          Log.i(TAG, "Initialized Controller. Local Address: $addr")
          localAddress.value = addr
        } else {
          val s = uwbManager.controleeSessionScope()
          controleeSession = s
          val addr = s.localAddress.address.toHexString()
          Log.i(TAG, "Initialized Controlee. Local Address: $addr")
          localAddress.value = addr
        }
        sessionState.value = "Initialized"
      } catch (e: Exception) {
        Log.e(TAG, "Initialization error: ${e.message}")
        sessionState.value = "Error: ${e.message}"
        e.printStackTrace()
      }
    }
  }

  @OptIn(ExperimentalStdlibApi::class)
  fun startRanging(peerAddressStrings: List<String>, configId: Int) {
    Log.i(TAG, "Starting UWB Ranging with peers $peerAddressStrings")

    rangingJob = scope.launch {
      try {
        // Initialize status map for these peers
        peerStatusMap.update { currentMap ->
          val newMap = currentMap.toMutableMap()
          peerAddressStrings.forEach { newMap[it] = "Connecting" }
          newMap
        }

        val complexChannel = UwbComplexChannel(currentChannel, currentPreambleIndex)
        val peerDevices = peerAddressStrings.map { UwbDevice(UwbAddress(it.hexToByteArray())) }

        val rangingParameters =
          RangingParameters(
            uwbConfigType = configId,
            sessionId = currentSessionId.toInt(),
            sessionKeyInfo = currentSessionKey,
            complexChannel = complexChannel,
            peerDevices = peerDevices,
            updateRateType = RangingParameters.RANGING_UPDATE_RATE_FREQUENT,
          )

        Log.i(TAG, "Preparing session with parameters: $rangingParameters")

        val sessionFlow =
          if (controllerSession != null) {
            controllerSession!!.prepareSession(rangingParameters)
          } else if (controleeSession != null) {
            controleeSession!!.prepareSession(rangingParameters)
          } else {
            throw IllegalStateException("Session not initialized! Call initSession first.")
          }

        // Radio is active and searching, but we aren't "Ranging" (receiving data) yet.
        sessionState.value = "Active"
        Log.i(TAG, "Collecting Ranging Results...")

        sessionFlow.collect { result: RangingResult ->
          Log.d(TAG, "Received RangingResult: $result")
          processResult(result)
        }
      } catch (e: kotlinx.coroutines.CancellationException) {
        Log.i(TAG, "Ranging coroutine cancelled")
      } catch (e: Exception) {
        Log.e(TAG, "Ranging error: ${e.message}")
        sessionState.value = "Error: ${e.message}"
        e.printStackTrace()
      }
    }
  }

  @OptIn(ExperimentalStdlibApi::class)
  private fun processResult(result: RangingResult) {
    when (result) {
      is RangingResult.RangingResultPosition -> {
        val peerAddr = result.device.address.address.toHexString()
        val dist = result.position.distance?.value
        val az = result.position.azimuth?.value
        val el = result.position.elevation?.value

        Log.i(TAG, "Ranging Result from $peerAddr: dist=$dist az=$az el=$el")

        // Once we get measurements, we are officially "Ranging"
        if (sessionState.value == "Active") {
          sessionState.value = "Ranging"
        }

        // Update peer status to Connected
        peerStatusMap.update { it + (peerAddr to "Connected") }

        if (dist != null) {
          lastDistanceMap.update { it + (peerAddr to dist) }
        }
        if (az != null) {
          lastAzimuthMap.update { it + (peerAddr to az) }
        }
        if (el != null) {
          lastElevationMap.update { it + (peerAddr to el) }
        }
      }
      is RangingResult.RangingResultPeerDisconnected -> {
        val peerAddr = result.device.address.address.toHexString()
        Log.i(TAG, "Peer Disconnected: $peerAddr")
        peerStatusMap.update { it + (peerAddr to "Disconnected") }

        // Optional: If all peers are disconnected, we could set sessionState to Disconnected
        if (peerStatusMap.value.values.all { it == "Disconnected" }) {
          sessionState.value = "Disconnected"
        }
      }
    }
  }

  fun stopRanging() {
    if (rangingJob != null) {
      Log.i(TAG, "Stopping UWB ranging")
      rangingJob?.cancel()
      rangingJob = null
      sessionState.value = "Stopped"
    }
    controllerSession = null
    controleeSession = null
    localAddress.value = null
    lastDistanceMap.value = emptyMap()
    lastAzimuthMap.value = emptyMap()
    lastElevationMap.value = emptyMap()
    peerStatusMap.value = emptyMap()
  }

  fun reset(context: Context) {
    stopRanging()
  }

  fun setParameters(channel: Int, preambleIndex: Int, sessionId: UInt, sessionKey: ByteArray) {
    currentChannel = channel
    currentPreambleIndex = preambleIndex
    currentSessionId = sessionId
    currentSessionKey = sessionKey
  }
}
