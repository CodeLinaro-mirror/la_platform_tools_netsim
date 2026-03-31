/*
 * Copyright 2026 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */
package com.android.netsim.agent

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.util.Log
import java.net.ServerSocket

private const val TAG = "ServiceDiscovery"

/// STEP: ^advertises service (.*) as (.*) on port (\d+)$
fun advertiseServiceWithPort(context: Context, args: List<String>): Map<String, String> {
  val serviceType = args[0]
  val serviceName = args[1]
  val port = args[2].toInt()
  return startNsd(context, serviceName, serviceType, port)
}

/// STEP: ^advertises service (.*) as (.*)$
fun advertiseService(context: Context, args: List<String>): Map<String, String> {
  val serviceType = args[0]
  val serviceName = args[1]
  return startNsd(context, serviceName, serviceType, 0)
}

/// STEP: ^starts discovery for (.*)$
fun startDiscovery(context: Context, args: List<String>) {
  val serviceType = args[0]
  DiscoveryState.reset(context)
  discoverNsd(context, serviceType)
}

/// STEP: ^finds service (.*)$
fun findService(context: Context, args: List<String>) {
  val expectedName = args[0]
  val start = System.currentTimeMillis()
  var found = false
  while (System.currentTimeMillis() - start < 10000) {
    if (DiscoveryState.hasFound(expectedName)) {
      found = true
      break
    }
    Thread.sleep(500)
  }
  if (found) {
    Log.i(TAG, "INFO Verifies that service $expectedName was discovered")
  } else {
    val foundServices = DiscoveryState.getAllFound()
    Log.i(TAG, "INFO  Timeout waiting for service $expectedName. Found: $foundServices")
    throw Exception("Service not found: $expectedName. Found: $foundServices")
  }
}

object DiscoveryState {
  private val foundServices = mutableSetOf<String>()
  val activeRegistrations = mutableListOf<NsdManager.RegistrationListener>()
  private val lock = Object()
  private var multicastLock: android.net.wifi.WifiManager.MulticastLock? = null
  var nsdManagerForCleanup: NsdManager? = null

  fun reset(context: Context) {
    synchronized(lock) {
      foundServices.clear()
      nsdManagerForCleanup?.let { manager ->
        val latch = java.util.concurrent.CountDownLatch(activeRegistrations.size)
        activeRegistrations.forEach { listener ->
          if (listener is TrackedRegistrationListener) {
            listener.cleanupLatch = latch
          }
          try {
            manager.unregisterService(listener)
          } catch (e: Exception) {
            Log.e(TAG, "Error unregistering service during reset", e)
            latch.countDown()
          }
        }
        try {
          latch.await(3, java.util.concurrent.TimeUnit.SECONDS)
        } catch (e: Exception) {
          Log.e(TAG, "Interrupted while waiting for NSD unregistration")
        }
      }
      activeRegistrations.clear()
    }
  }

  fun hasFound(name: String): Boolean {
    synchronized(lock) {
      return foundServices.any { it.contains(name) }
    }
  }

  fun getAllFound(): String {
    synchronized(lock) {
      return foundServices.joinToString(", ")
    }
  }

  fun setFound(name: String) {
    synchronized(lock) { foundServices.add(name) }
  }

  fun acquireLock(context: Context) {
    synchronized(lock) {
      if (multicastLock == null) {
        val wifiManager =
          context.getSystemService(Context.WIFI_SERVICE) as android.net.wifi.WifiManager
        multicastLock = wifiManager.createMulticastLock("NTestDiscovery")
        multicastLock?.setReferenceCounted(true)
      }
      multicastLock?.acquire()
    }
  }

  fun releaseLock() {
    synchronized(lock) {
      if (multicastLock?.isHeld == true) {
        multicastLock?.release()
      }
    }
  }
}

private fun startNsd(
  context: Context,
  serviceName: String,
  serviceType: String,
  requestedPort: Int,
): Map<String, String> {
  val nsdManager = context.getSystemService(Context.NSD_SERVICE) as NsdManager
  val port =
    if (requestedPort > 0) {
      requestedPort
    } else {
      try {
        ServerSocket(0).use { it.localPort }
      } catch (e: Exception) {
        0
      }
    }

  val serviceInfo =
    NsdServiceInfo().apply {
      this.serviceName = "$serviceName-${java.util.UUID.randomUUID().toString().take(8)}"
      this.serviceType = serviceType
      this.port = port
    }

  val maxRetries = 3
  for (attempt in 1..maxRetries) {
    val latch = java.util.concurrent.CountDownLatch(1)
    var error: Int? = null

    val listener =
      object : TrackedRegistrationListener() {
        override fun onRegistrationFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
          Log.i(TAG, "Registration failed: $errorCode on attempt $attempt")
          error = errorCode
          latch.countDown()
        }

        override fun onUnregistrationFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
          cleanupLatch?.countDown()
        }

        override fun onServiceRegistered(serviceInfo: NsdServiceInfo) {
          Log.i(TAG, "INFO Verifies the service is registered")
          latch.countDown()
        }

        override fun onServiceUnregistered(serviceInfo: NsdServiceInfo) {
          cleanupLatch?.countDown()
        }
      }

    synchronized(DiscoveryState) {
      DiscoveryState.nsdManagerForCleanup = nsdManager
      DiscoveryState.activeRegistrations.add(listener)
    }

    nsdManager.registerService(serviceInfo, NsdManager.PROTOCOL_DNS_SD, listener)

    if (!latch.await(5, java.util.concurrent.TimeUnit.SECONDS)) {
      if (attempt == maxRetries)
        throw Exception("Timeout waiting for NSD registration of $serviceName")
      continue
    }

    if (error == null) {
      return mapOf("port" to port.toString())
    }

    // Error 0 usually means an internal error, often caused by a previous
    // test's unregistration being processed asynchronously by the OS.
    if (attempt == maxRetries) {
      throw Exception("NSD Registration Failed: $error after $maxRetries attempts")
    }

    Log.i(TAG, "Retrying NSD Registration after Error $error. Waiting 1s...")
    Thread.sleep(1000)
  }

  throw Exception("NSD Registration completely failed.")
}

private fun discoverNsd(context: Context, serviceType: String) {
  DiscoveryState.acquireLock(context)
  val nsdManager = context.getSystemService(Context.NSD_SERVICE) as NsdManager

  val latch = java.util.concurrent.CountDownLatch(1)
  var error: Int? = null

  nsdManager.discoverServices(
    serviceType,
    NsdManager.PROTOCOL_DNS_SD,
    object : NsdManager.DiscoveryListener {
      override fun onDiscoveryStarted(regType: String) {
        Log.i(TAG, "Discovery started: $regType")
        latch.countDown()
      }

      override fun onServiceFound(serviceInfo: NsdServiceInfo) {
        Log.i(TAG, "Service found: ${serviceInfo.serviceName}")
        DiscoveryState.setFound(serviceInfo.serviceName)
      }

      override fun onServiceLost(serviceInfo: NsdServiceInfo) {
        Log.i(TAG, "Service lost: ${serviceInfo.serviceName}")
      }

      override fun onDiscoveryStopped(serviceType: String) {
        Log.i(TAG, "Discovery stopped: $serviceType")
        DiscoveryState.releaseLock()
      }

      override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
        Log.i(TAG, "Discovery failed to start: $errorCode")
        nsdManager.stopServiceDiscovery(this)
        DiscoveryState.releaseLock()
        error = errorCode
        latch.countDown()
      }

      override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) {
        Log.i(TAG, "Stop discovery failed: $errorCode")
        nsdManager.stopServiceDiscovery(this)
        DiscoveryState.releaseLock()
      }
    },
  )

  if (!latch.await(5, java.util.concurrent.TimeUnit.SECONDS)) {
    throw Exception("Timeout waiting for discovery start")
  }
  error?.let { throw Exception("Discovery Start Failed: $it") }
}

abstract class TrackedRegistrationListener : NsdManager.RegistrationListener {
  var cleanupLatch: java.util.concurrent.CountDownLatch? = null
}
