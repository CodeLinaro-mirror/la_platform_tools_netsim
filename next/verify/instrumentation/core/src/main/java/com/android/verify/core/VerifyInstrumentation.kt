/*
 * Copyright 2026 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */

package com.android.verify.core

import android.app.Instrumentation
import android.content.Context
import android.os.Bundle
import android.util.Log
import androidx.test.platform.app.InstrumentationRegistry

/**
 * VerifyInstrumentation is a specialized Android Instrumentation class that acts as a base for the
 * verify orchestrator. It executes HDD steps received over a TCP control channel (reverse tunneled
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

  private val observables = java.util.concurrent.CopyOnWriteArrayList<FeatureObservable>()

  fun registerObservable(observable: FeatureObservable) {
    observables.add(observable)
  }

  /**
   * Collects observables from all registered collectors. Note: This method is expected to be called
   * infrequently (e.g., once per verification step), so the allocation of a new map on each call is
   * acceptable.
   */
  fun getCollectedObservables(): Map<String, String> {
    val result = mutableMapOf<String, String>()
    for (obs in observables) {
      result.putAll(obs.getObservables())
    }
    return result
  }

  @Volatile protected var controlOutputStream: java.io.DataOutputStream? = null
  private val logExecutor = java.util.concurrent.Executors.newSingleThreadExecutor()

  fun log(msg: String) {
    Log.i(TAG, msg)
    logExecutor.execute {
      try {
        controlOutputStream?.let { dos ->
          val json = org.json.JSONObject()
          json.put("type", "Event")
          json.put("event_type", "Log")
          json.put("message", msg)
          val bytes = json.toString().toByteArray(Charsets.UTF_8)
          dos.writeShort(bytes.size)
          dos.write(bytes)
          dos.flush()
        }
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
            controlOutputStream = java.io.DataOutputStream(finalSocket.getOutputStream())
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
    val wait = arguments.getString("wait") == "true"

    if (initialStep != null) {
      registry?.execute(initialStep)
    }
    if (!wait) {
      finish(0, Bundle())
    }
  }

  /**
   * Starts the bidirectional control loop on a dedicated thread. Listens for raw HDD strings from
   * the Host-side orchestrator.
   */
  private fun startControlLoop(socket: java.net.Socket) {
    java.lang
      .Thread(
        {
          try {
            val dis = java.io.DataInputStream(socket.getInputStream())
            while (true) {
              val length =
                try {
                  dis.readUnsignedShort()
                } catch (e: java.io.EOFException) {
                  break // Connection closed
                }
              val bytes = ByteArray(length)
              dis.readFully(bytes)
              val jsonStr = String(bytes, Charsets.UTF_8)
              val json = org.json.JSONObject(jsonStr)

              val type = json.optString("type")
              if (type == "Quit") {
                val id = json.optInt("id")
                sendResponse(id, "Success", null, null)
                finish(0, Bundle())
                break
              } else if (type == "GetSteps") {
                val id = json.optInt("id")
                val r = registry
                val steps = r?.getRegisteredRegexes() ?: emptyList()
                sendStepsResponse(id, steps)
              } else if (type == "ExecuteStep") {
                val id = json.optInt("id")
                val cmd = json.optString("step")

                val r = registry
                if (r == null) {
                  sendResponse(id, "Failure", "Registry not initialized", null)
                  continue
                }

                try {
                  val (found, result) = r.execute(cmd)
                  if (found) {
                    val variables =
                      if (result is Map<*, *>) {
                        val map = HashMap<String, String>()
                        result.forEach { (k, v) -> map[k.toString()] = v.toString() }
                        map
                      } else null
                    sendResponse(id, "Success", null, variables)
                  } else {
                    sendResponse(id, "Failure", "No matching step found", null)
                  }
                } catch (e: QuitException) {
                  sendResponse(id, "Success", null, null)
                  finish(0, Bundle())
                  break
                } catch (e: Exception) {
                  sendResponse(id, "Failure", e.message ?: "Unknown error", null)
                }
              } else if (type == "StartScenario") {
                val id = json.optInt("id")
                Log.i(TAG, "Starting scenario")
                sendResponse(id, "Success", null, null)
              } else if (type == "StopScenario") {
                val id = json.optInt("id")
                Log.i(TAG, "Stopping scenario, performing cleanup")
                sendResponse(id, "Success", null, null)
              }
            }
          } catch (e: Exception) {
            Log.e(TAG, "Error in control loop: ${e.message}")
          }
        },
        "VerifyControlLoop",
      )
      .start()
  }

  private fun sendResponse(
    id: Int,
    status: String,
    errorMessage: String?,
    variables: Map<String, String>?,
  ) {
    logExecutor.execute {
      try {
        controlOutputStream?.let { dos ->
          val json = org.json.JSONObject()
          json.put("type", "Response")
          json.put("id", id)
          json.put("status", status)
          if (errorMessage != null) {
            json.put("error_message", errorMessage)
          }
          if (variables != null) {
            val varsJson = org.json.JSONObject()
            variables.forEach { (k, v) -> varsJson.put(k, v) }
            json.put("variables", varsJson)
          }
          val bytes = json.toString().toByteArray(Charsets.UTF_8)
          dos.writeShort(bytes.size)
          dos.write(bytes)
          dos.flush()
        }
      } catch (e: Exception) {
        Log.e(TAG, "Failed to send response: ${e.message}")
      }
    }
  }

  private fun sendStepsResponse(id: Int, steps: List<String>) {
    logExecutor.execute {
      try {
        controlOutputStream?.let { dos ->
          val json = org.json.JSONObject()
          json.put("type", "StepsResponse")
          json.put("id", id)
          val stepsJson = org.json.JSONArray()
          steps.forEach { stepsJson.put(it) }
          json.put("steps", stepsJson)
          val bytes = json.toString().toByteArray(Charsets.UTF_8)
          dos.writeShort(bytes.size)
          dos.write(bytes)
          dos.flush()
        }
      } catch (e: Exception) {
        Log.e(TAG, "Failed to send steps response: ${e.message}")
      }
    }
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

fun Context.logToHost(msg: String) {
  val instr = InstrumentationRegistry.getInstrumentation() as? VerifyInstrumentation
  instr?.log(msg)
}
