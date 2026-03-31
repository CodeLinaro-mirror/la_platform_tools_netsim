/*
 * Copyright 2026 The Android Open Source Project
 */

package com.android.netsim.agent

import android.app.Instrumentation
import android.os.Bundle
import android.util.Log

/**
 * NTestInstrumentation is a specialized Android Instrumentation class that acts as a "Guest Agent"
 * for the ntest orchestrator. It executes BDD steps received over a TCP control channel (reverse
 * tunneled via ADB) and reports results back to the host.
 */
class NTestInstrumentation : Instrumentation() {

  // Guest-Host Communication Protocol
  private object Protocol {
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

  private val TAG = "NTestAgent"

  @Volatile private var registry: StepRegistry? = null
  // private lateinit var nsdManager: NsdManager
  private var controlWriter: java.io.PrintWriter? = null

  private fun log(msg: String) {
    Log.i(TAG, msg)
    controlWriter?.println(msg)
    controlWriter?.flush()
  }

  override fun onCreate(arguments: Bundle) {
    Log.i(TAG, "NTestInstrumentation.onCreate starting")
    super.onCreate(arguments)

    val deviceName = arguments.getString("device_name") ?: "Android"
    val controlPort = arguments.getString("control_port")?.toIntOrNull() ?: 0

    val context = getContext()
    // nsdManager no longer needed here
    registry = StepRegistry(context)
    registerSteps()

    if (controlPort > 0) {
      java.lang
        .Thread {
          var connected = false
          var retries = 0
          try {
            val socket: java.net.Socket =
              retryUntil(
                maxAttempts = MAX_RETRIES,
                delayMillis = RETRY_DELAY_MS,
                errorMessage =
                  "Failed to connect to control port $controlPort after $MAX_RETRIES attempts",
                action = {
                  try {
                    Log.i(
                      TAG,
                      "Attempting to connect to control port: $controlPort (attempt ${retries + 1})",
                    )
                    val s = java.net.Socket("127.0.0.1", controlPort)
                    Log.i(TAG, "Successfully connected to control port: $controlPort")
                    s
                  } catch (e: Exception) {
                    Log.e(TAG, "Failed to connect to control port $controlPort: ${e.message}")
                    retries++
                    null
                  }
                },
                condition = { it != null },
              )!! // Safe because retryUntil throws if condition not met, and condition checks for
            // non-null

            connected = true
            controlWriter = java.io.PrintWriter(socket.getOutputStream(), true)
            startControlLoop(socket)
          } catch (e: IllegalStateException) {
            Log.e(TAG, "Giving up on control port $controlPort after $MAX_RETRIES attempts")
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
        "NTestControlLoop",
      )
      .start()
  }

  private fun registerSteps() {
    val r = registry ?: return
    val context = getContext()

    WifiStepsLoader.loadSteps(r, context)
    NetworkStepsLoader.loadSteps(r, context)
    LifecycleStepsLoader.loadSteps(r, context)
    ServiceDiscoveryLoader.loadSteps(r, context)
    TcpEchoStepsLoader.loadSteps(r, context)
    UwbStepsLoader.loadSteps(r, context)
  }
}

fun <T> retryUntil(
  maxAttempts: Int = 50,
  delayMillis: Long = 100L,
  errorMessage: String = "Condition not met after $maxAttempts attempts.",
  action: () -> T,
  condition: (T) -> Boolean,
): T {
  repeat(maxAttempts) { attempt ->
    val result = action()

    if (condition(result)) {
      return result
    }

    if (attempt < maxAttempts - 1) {
      Thread.sleep(delayMillis)
    }
  }

  throw IllegalStateException(errorMessage)
}
