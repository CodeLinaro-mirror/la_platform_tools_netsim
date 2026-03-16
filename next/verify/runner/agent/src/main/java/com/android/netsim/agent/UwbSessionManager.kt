/*
 * Copyright 2026 The Android Open Source Project
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
import kotlinx.coroutines.launch

@OptIn(ExperimentalStdlibApi::class)
object UwbSessionManager {
  private const val TAG = "UwbSessionManager"
  private var rangingJob: Job? = null
  private val scope = CoroutineScope(Dispatchers.IO)

  // Last observed results for verification steps
  val lastDistance = MutableStateFlow<Float?>(null)
  val lastAzimuth = MutableStateFlow<Float?>(null)
  val lastElevation = MutableStateFlow<Float?>(null)

  val sessionState = MutableStateFlow<String>("Idle")

  // Default parameters (can be overridden by steps)
  var currentChannel = 9
  var currentPreambleIndex = 9
  var currentSessionId = 12345678
  var currentSessionKey = byteArrayOf(0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08)

  // Stored scopes for split initialization
  private var controllerSession: androidx.core.uwb.UwbControllerSessionScope? = null
  private var controleeSession: androidx.core.uwb.UwbControleeSessionScope? = null

  // Exposed local address for OOB simulation
  val localAddress = MutableStateFlow<String?>(null)

  fun initSession(context: Context, isController: Boolean) {
    stopRanging()
    sessionState.value = "Initializing"
    localAddress.value = null
    controllerSession = null
    controleeSession = null

    val uwbManager = UwbManager.createInstance(context)

    rangingJob =
      scope.launch {
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
  fun startRanging(peerAddressStr: String, configId: Int) {
    Log.i(TAG, "Starting UWB Ranging with peer $peerAddressStr")

    rangingJob =
      scope.launch {
        try {
          val complexChannel = UwbComplexChannel(currentChannel, currentPreambleIndex)
          val peerAddress = UwbAddress(peerAddressStr.hexToByteArray())

          val rangingParameters =
            RangingParameters(
              uwbConfigType = configId,
              sessionId = currentSessionId,
              sessionKeyInfo = currentSessionKey,
              complexChannel = complexChannel,
              peerDevices = listOf(UwbDevice(peerAddress)),
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

          sessionState.value = "Ranging"

          sessionFlow.collect { result: RangingResult -> processResult(result) }
        } catch (e: Exception) {
          Log.e(TAG, "Ranging error: ${e.message}")
          sessionState.value = "Error: ${e.message}"
          e.printStackTrace()
        }
      }
  }

  fun startController(context: Context, peerAddressStr: String, configId: Int) {
    scope.launch {
      initSession(context, true)
      var retries = 50
      while (localAddress.value == null && retries > 0) {
        kotlinx.coroutines.delay(100)
        retries--
      }
      if (localAddress.value != null) {
        startRanging(peerAddressStr, configId)
      }
    }
  }

  fun startControlee(context: Context, peerAddressStr: String, configId: Int) {
    scope.launch {
      initSession(context, false)
      var retries = 50
      while (localAddress.value == null && retries > 0) {
        kotlinx.coroutines.delay(100)
        retries--
      }
      if (localAddress.value != null) {
        startRanging(peerAddressStr, configId)
      }
    }
  }

  private fun processResult(result: RangingResult) {
    when (result) {
      is RangingResult.RangingResultPosition -> {
        val dist = result.position.distance?.value
        val az = result.position.azimuth?.value
        val el = result.position.elevation?.value

        Log.i(TAG, "Ranging Result: dist=$dist az=$az el=$el")

        if (dist != null) lastDistance.value = dist
        if (az != null) lastAzimuth.value = az
        if (el != null) lastElevation.value = el
      }
      is RangingResult.RangingResultPeerDisconnected -> {
        Log.i(TAG, "Peer Disconnected")
        sessionState.value = "Disconnected"
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
  }

  fun setParameters(channel: Int, preambleIndex: Int, sessionId: Int, sessionKey: ByteArray) {
    currentChannel = channel
    currentPreambleIndex = preambleIndex
    currentSessionId = sessionId
    currentSessionKey = sessionKey
  }
}
