/*
 * Copyright 2026 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */

package com.android.verify.core

import android.app.Instrumentation
import android.os.Bundle
import android.util.Log
import androidx.test.platform.app.InstrumentationRegistry

/**
 * VerifyInstrumentation is a specialized Android Instrumentation class that acts as a base for the
 * verify orchestrator. It executes BDD steps received over a TCP control channel (reverse tunneled
 * via ADB) and reports results back to the host.
 */
open class VerifyInstrumentation : Instrumentation() {

  // Guest-Host Communication Protocol
  protected object Protocol {
    const val MARKER_RECEIVED = ">> RECEIVED:"
    const val MARKER_COMPLETED = "<< COMPLETED:"
    const val RESULT_SUCCESS = "RESULT=SUCCESS"
    const val RESULT_FAILURE = "RESULT=FAILURE"
    const val MSG_KEY = "MSG="
    const val INFO_TAG = "INFO"
  }

  companion object {
    private const val MAX_RETRIES = 60
    private const val RETRY_DELAY_MS = 1000L
  }

  private val TAG = "VerifyInstrumentation"

  @Volatile protected var registry: StepRegistry? = null
  private var controlWriter: java.io.PrintWriter? = null
  private val logExecutor = java.util.concurrent.Executors.newSingleThreadExecutor()

  protected fun log(msg: String) {
    Log.i(TAG, msg)
    logExecutor.execute {
      try {
        controlWriter?.println(msg)
        controlWriter?.flush()
      } catch (e: Exception) {
        Log.e(TAG, "Failed to write to control socket: ${e.message}")
      }
    }
  }

  override fun onCreate(arguments: Bundle) {
    Log.i(TAG, "VerifyInstrumentation.onCreate starting")
    super.onCreate(arguments)
    InstrumentationRegistry.registerInstance(this, arguments)

    val deviceName = arguments.getString("device_name") ?: "Android"
    val controlPort = arguments.getString("control_port")?.toIntOrNull() ?: 0

    val context = getContext()
    registry = StepRegistry(context)

    // Grant permissions
    try {
      val pkg = context.packageName
      val uiAutomation = getUiAutomation()

      val perms =
        listOf(
          "android.permission.BLUETOOTH_ADVERTISE",
          "android.permission.BLUETOOTH_SCAN",
          "android.permission.BLUETOOTH_CONNECT",
          "android.permission.ACCESS_FINE_LOCATION",
          "android.permission.ACCESS_COARSE_LOCATION",
          "android.permission.UWB_RANGING",
        )

      for (perm in perms) {
        uiAutomation.executeShellCommand("pm grant $pkg $perm")
      }

      // Application Operations (AppOps)
      try {
        uiAutomation.executeShellCommand("appops set $pkg FINE_LOCATION allow")
        uiAutomation.executeShellCommand("appops set $pkg COARSE_LOCATION allow")
        uiAutomation.executeShellCommand("appops set $pkg BLUETOOTH_SCAN allow")
        uiAutomation.executeShellCommand("appops set $pkg BLUETOOTH_ADVERTISE allow")
        uiAutomation.executeShellCommand("appops set $pkg BLUETOOTH_CONNECT allow")
        uiAutomation.executeShellCommand("appops set $pkg UWB_RANGING allow")
      } catch (ignore: Exception) {
        Log.w(TAG, "Failed to set AppOps: ${ignore.message}")
      }

      // Give it a moment to propagate
      java.lang.Thread.sleep(500)
    } catch (e: Exception) {
      Log.e(TAG, "Failed to grant permissions: ${e.message}")
    }

    registerSteps()

    if (controlPort > 0) {
      java.lang
        .Thread {
          try {
            val finalSocket =
              retryUntil(
                maxAttempts = MAX_RETRIES,
                delayMillis = RETRY_DELAY_MS,
                errorMessage =
                  "Failed to connect to control port $controlPort after $MAX_RETRIES attempts",
                action = { attempt ->
                  Log.i(
                    TAG,
                    "Attempting to connect to control port: $controlPort (attempt ${attempt + 1})",
                  )
                  try {
                    java.net.Socket("127.0.0.1", controlPort)
                  } catch (e: Exception) {
                    Log.e(TAG, "Failed to connect to control port $controlPort: ${e.message}")
                    null
                  }
                },
                condition = { it != null },
              )!!
            Log.i(TAG, "Successfully connected to control port: $controlPort")
            controlWriter = java.io.PrintWriter(finalSocket.getOutputStream(), true)
            startControlLoop(finalSocket)
          } catch (e: Exception) {
            Log.e(
              TAG,
              "Giving up on control port $controlPort after $MAX_RETRIES attempts: ${e.message}",
            )
          }
        }
        .start()
    } else {
      Log.w(TAG, "No control_port provided or invalid: ${arguments.getString("control_port")}")
    }

    val initialStep = arguments.getString("step")

    if (initialStep != null) {
      registry?.execute(initialStep)
    }

    val wait = arguments.getString("wait") == "true"
    if (!wait) {
      finish(0, Bundle())
    }
  }

  /**
   * Starts the bidirectional control loop on a dedicated thread. Listens for raw BDD strings from
   * the Host-side orchestrator.
   */
  private fun startControlLoop(socket: java.net.Socket) {
    java.lang
      .Thread(
        {
          try {
            val reader = socket.getInputStream().bufferedReader()
            for (line in reader.lineSequence()) {
              if (line.isBlank()) continue
              val cmd = line.trim()

              log("${Protocol.MARKER_RECEIVED} $cmd")
              val r = registry
              if (r == null) {
                log("${Protocol.INFO_TAG} Registry not initialized")
                continue
              }

              try {
                val (found, result) = r.execute(cmd)
                if (found) {
                  val varStr =
                    if (result is Map<*, *>) {
                      result.entries.joinToString(" ") { "VAR:${it.key}=${it.value}" }
                    } else {
                      if (result != null) "VAR:DEBUG_TYPE=${result.javaClass.name}" else ""
                    }
                  log("${Protocol.MARKER_COMPLETED} $cmd ${Protocol.RESULT_SUCCESS} $varStr")
                } else {
                  log(
                    "${Protocol.MARKER_COMPLETED} $cmd ${Protocol.RESULT_FAILURE} ${Protocol.MSG_KEY}No matching step found"
                  )
                }
              } catch (e: QuitException) {
                log("${Protocol.MARKER_COMPLETED} $cmd ${Protocol.RESULT_SUCCESS}")
                finish(0, Bundle())
              } catch (e: Exception) {
                log(
                  "${Protocol.MARKER_COMPLETED} $cmd ${Protocol.RESULT_FAILURE} ${Protocol.MSG_KEY}${e.message}"
                )
              }
            }
          } catch (e: Exception) {
            // Connection closed or I/O error; terminate the control loop
          }
        },
        "VerifyControlLoop",
      )
      .start()
  }

  /** To be overridden by subclasses to register specific steps. */
  open fun registerSteps() {
    // Default implementation does nothing
  }
}

fun <T> retryUntil(
  maxAttempts: Int,
  delayMillis: Long,
  errorMessage: String,
  action: (Int) -> T,
  condition: (T) -> Boolean,
): T {
  repeat(maxAttempts) { attempt ->
    val result = action(attempt)
    if (condition(result)) {
      return result
    }
    if (attempt < maxAttempts - 1) {
      Thread.sleep(delayMillis)
    }
  }
  throw IllegalStateException(errorMessage)
}
