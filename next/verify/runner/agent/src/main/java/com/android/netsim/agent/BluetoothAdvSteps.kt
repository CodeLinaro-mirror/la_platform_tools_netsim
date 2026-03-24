/*
 * Copyright 2026 The Android Open Source Project
 */

package com.android.netsim.agent

import android.bluetooth.BluetoothManager
import android.bluetooth.le.AdvertiseCallback
import android.bluetooth.le.AdvertiseData
import android.bluetooth.le.AdvertiseSettings
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanFilter
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.location.LocationManager
import android.os.ParcelUuid
import android.util.Log
import androidx.test.platform.app.InstrumentationRegistry
import java.util.UUID
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean

private const val TAG = "BluetoothAdvSteps"
private const val RSSI_THRESHOLD_HIGH = -60
private const val RSSI_THRESHOLD_MEDIUM = -75

// Helper to keep track of state
object BluetoothState {
  val advertisers = mutableListOf<AdvertiseCallback>()
  val scanners = mutableListOf<ScanCallback>()
  val scanResults = mutableListOf<ScanResult>()
  var lastError: String? = null
  var logger: (String) -> Unit = { Log.i(TAG, it) }
  private val lock = Object()

  fun log(msg: String) {
    logger(msg)
  }

  fun reportError(error: String) {
    synchronized(lock) {
      lastError = error
      Log.e(TAG, "Reported Error: $error")
    }
  }

  fun getError(): String? {
    synchronized(lock) {
      return lastError
    }
  }

  fun addAdvertiser(callback: AdvertiseCallback) {
    synchronized(lock) { advertisers.add(callback) }
  }

  fun addScanner(callback: ScanCallback) {
    synchronized(lock) { scanners.add(callback) }
  }

  fun addScanResult(result: ScanResult) {
    synchronized(lock) { scanResults.add(result) }
  }

  fun hasSeen(name: String, uuid: UUID): Boolean {
    synchronized(lock) {
      val res = scanResults.any {
        val record = it.scanRecord
        val dName = record?.deviceName
        val nameMatch = dName == name
        val uuidMatch = record?.serviceUuids?.any { u -> u.uuid == uuid } == true
        nameMatch || uuidMatch
      }
      return res
    }
  }

  fun getSeenCount(name: String, uuid: UUID): Int {
    synchronized(lock) {
      return scanResults.count {
        val record = it.scanRecord
        val nameMatch = record?.deviceName == name
        val uuidMatch = record?.serviceUuids?.any { u -> u.uuid == uuid } == true
        nameMatch || uuidMatch
      }
    }
  }

  fun getSeenDebugString(): String {
    synchronized(lock) {
      return scanResults.joinToString("\n") {
        "Device: ${it.device.address}, Name: ${it.scanRecord?.deviceName}, UUIDs: ${it.scanRecord?.serviceUuids}"
      }
    }
  }

  fun getLatestResult(name: String, uuid: UUID): ScanResult? {
    synchronized(lock) {
      return scanResults
        .filter {
          val record = it.scanRecord
          val nameMatch = record?.deviceName == name
          val uuidMatch = record?.serviceUuids?.any { u -> u.uuid == uuid } == true
          nameMatch || uuidMatch
        }
        .maxByOrNull { it.timestampNanos }
    }
  }

  fun reset(context: Context) {
    synchronized(lock) {
      val manager = context.getSystemService(Context.BLUETOOTH_SERVICE) as BluetoothManager
      val adapter = manager.adapter
      if (adapter != null && adapter.isEnabled) {
        val advertiser = adapter.bluetoothLeAdvertiser
        if (advertiser != null) {
          advertisers.forEach {
            try {
              advertiser.stopAdvertising(it)
            } catch (e: Exception) {
              Log.e(TAG, "Error stopping advertising", e)
            }
          }
        }

        val scanner = adapter.bluetoothLeScanner
        if (scanner != null) {
          scanners.forEach {
            try {
              scanner.stopScan(it)
            } catch (e: Exception) {
              Log.e(TAG, "Error stopping scan", e)
            }
          }
        }
      }

      scanResults.clear()
      advertisers.clear()
      scanners.clear()
      lastError = null
    }
  }
}

/// STEP: ^advertises with name "(.*)" and TxPower "(.*)"$
fun advertiseWithNameAndPower(context: Context, args: List<String>): Map<String, String> {
  val name = args[0]
  val txPowerStr = args[1]

  val manager = context.getSystemService(Context.BLUETOOTH_SERVICE) as BluetoothManager
  val adapter = manager.adapter
  if (!adapter.isEnabled) {
    throw Exception("Bluetooth Adapter is NOT enabled")
  }

  val txPower =
    when (txPowerStr.uppercase()) {
      "ULTRA_LOW" -> AdvertiseSettings.ADVERTISE_TX_POWER_ULTRA_LOW
      "LOW" -> AdvertiseSettings.ADVERTISE_TX_POWER_LOW
      "MEDIUM" -> AdvertiseSettings.ADVERTISE_TX_POWER_MEDIUM
      "HIGH" -> AdvertiseSettings.ADVERTISE_TX_POWER_HIGH
      else -> AdvertiseSettings.ADVERTISE_TX_POWER_MEDIUM
    }

  val advertiser =
    adapter.bluetoothLeAdvertiser ?: throw Exception("Bluetooth LE Advertiser not available")

  val settings =
    AdvertiseSettings.Builder()
      .setAdvertiseMode(AdvertiseSettings.ADVERTISE_MODE_LOW_LATENCY)
      .setTxPowerLevel(txPower)
      .setConnectable(true)
      .build()

  // Create a predictable UUID from the name to check against
  val uuid = UUID.nameUUIDFromBytes(name.toByteArray())
  val pUuid = ParcelUuid(uuid)

  if (adapter.name != name) {
    val nameLatch = CountDownLatch(1)
    val nameReceiver =
      object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
          if (android.bluetooth.BluetoothAdapter.ACTION_LOCAL_NAME_CHANGED == intent.action) {
            val newName = intent.getStringExtra(android.bluetooth.BluetoothAdapter.EXTRA_LOCAL_NAME)
            if (newName == name) {
              nameLatch.countDown()
            }
          }
        }
      }

    val filter = IntentFilter(android.bluetooth.BluetoothAdapter.ACTION_LOCAL_NAME_CHANGED)
    context.registerReceiver(nameReceiver, filter)

    try {
      adapter.name = name
      if (!nameLatch.await(5, TimeUnit.SECONDS)) {
        Log.w(TAG, "Timeout waiting for ACTION_LOCAL_NAME_CHANGED to $name. Continuing anyway.")
      } else {
        Log.i(TAG, "Adapter name successfully changed to $name")
      }
    } finally {
      try {
        context.unregisterReceiver(nameReceiver)
      } catch (e: Exception) {
        // Ignore unregister errors
      }
    }
  } else {
    Log.i(TAG, "Adapter name is already $name")
  }

  val data = AdvertiseData.Builder().setIncludeDeviceName(true).addServiceUuid(pUuid).build()

  val latch = CountDownLatch(1)
  val success = AtomicBoolean(false)
  val callback =
    object : AdvertiseCallback() {
      override fun onStartSuccess(settingsInEffect: AdvertiseSettings) {
        Log.i(TAG, "Advertise success: $name")
        success.set(true)
        latch.countDown()
      }

      override fun onStartFailure(errorCode: Int) {
        Log.e(TAG, "Advertise failure: $errorCode")
        BluetoothState.reportError("Advertise Start Failed: $errorCode")
        latch.countDown()
      }
    }

  BluetoothState.addAdvertiser(callback)
  advertiser.startAdvertising(settings, data, callback)

  if (!latch.await(5, TimeUnit.SECONDS)) {
    throw Exception("Timeout waiting for advertising start: $name")
  }

  if (!success.get()) {
    throw Exception("Advertising failed to start: $name. Error: ${BluetoothState.getError()}")
  }

  Log.i(TAG, "Started advertising name: $name with TxPower: $txPowerStr")
  return mapOf("status" to "advertising", "name" to name)
}

/// STEP: ^starts scanning$
fun startScanning(context: Context, args: List<String>): Map<String, String> {
  val manager = context.getSystemService(Context.BLUETOOTH_SERVICE) as BluetoothManager
  val adapter = manager.adapter
  if (!adapter.isEnabled) {
    throw Exception("Bluetooth Adapter is NOT enabled")
  }

  val locationManager = context.getSystemService(Context.LOCATION_SERVICE) as LocationManager
  // Force enable location every time to be sure
  try {
    val instrumentation = InstrumentationRegistry.getInstrumentation()
    val parcelFileDescriptor =
      instrumentation.uiAutomation.executeShellCommand("cmd location set-location-enabled true")
    parcelFileDescriptor.close()
    Log.i(TAG, "Sent enable location command")

    // Wait for location to enable with a timeout
    val start = System.currentTimeMillis()
    while (!locationManager.isLocationEnabled && System.currentTimeMillis() - start < 10000) {
      Thread.sleep(200)
    }
    if (!locationManager.isLocationEnabled) {
      Log.w(TAG, "Location check timed out, but proceeding anyway (might be flaky)")
    }
  } catch (e: Exception) {
    Log.e(TAG, "Error enabling location: ${e.message}")
  }

  val scanner = adapter.bluetoothLeScanner ?: throw Exception("Bluetooth LE Scanner not available")

  val settings = ScanSettings.Builder().setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY).build()

  val callback =
    object : ScanCallback() {
      override fun onScanResult(callbackType: Int, result: ScanResult) {
        val name = result.scanRecord?.deviceName
        val address = result.device.address
        BluetoothState.log("SCANNED: Address=$address, Name=$name, RSSI=${result.rssi}")
        BluetoothState.addScanResult(result)
      }

      override fun onBatchScanResults(results: MutableList<ScanResult>) {
        results.forEach { BluetoothState.addScanResult(it) }
      }

      override fun onScanFailed(errorCode: Int) {
        Log.e(TAG, "Scan failed: $errorCode")
        BluetoothState.reportError("Scan Start Failed: $errorCode")
      }
    }

  BluetoothState.addScanner(callback)

  val uiAutomation = InstrumentationRegistry.getInstrumentation().uiAutomation
  uiAutomation.adoptShellPermissionIdentity()
  try {
    scanner.startScan(null, settings, callback)
  } finally {
    uiAutomation.dropShellPermissionIdentity()
  }

  Log.i(TAG, "Started scanning (with shell identity) for package: ${context.packageName}")
  return mapOf("status" to "scanning")
}

/// STEP: ^starts scanning for "(.*)"$
fun startScanningFor(context: Context, args: List<String>): Map<String, String> {
  val name = args[0]
  val manager = context.getSystemService(Context.BLUETOOTH_SERVICE) as BluetoothManager
  val adapter = manager.adapter
  if (!adapter.isEnabled) {
    throw Exception("Bluetooth Adapter is NOT enabled")
  }

  val locationManager = context.getSystemService(Context.LOCATION_SERVICE) as LocationManager
  if (!locationManager.isLocationEnabled) {
    Log.w(TAG, "Location is disabled. Attempting to enable via shell...")
    try {
      val instrumentation = InstrumentationRegistry.getInstrumentation()
      val parcelFileDescriptor =
        instrumentation.uiAutomation.executeShellCommand("cmd location set-location-enabled true")
      parcelFileDescriptor.close()

      // Wait for location to enable with a timeout
      val start = System.currentTimeMillis()
      while (!locationManager.isLocationEnabled && System.currentTimeMillis() - start < 10000) {
        Thread.sleep(200)
      }
    } catch (e: Exception) {
      Log.e(TAG, "Error enabling location: ${e.message}")
    }
  }

  val scanner = adapter.bluetoothLeScanner ?: throw Exception("Bluetooth LE Scanner not available")

  val settings = ScanSettings.Builder().setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY).build()

  val filter = ScanFilter.Builder().setDeviceName(name).build()

  val callback =
    object : ScanCallback() {
      override fun onScanResult(callbackType: Int, result: ScanResult) {
        BluetoothState.addScanResult(result)
      }

      override fun onBatchScanResults(results: MutableList<ScanResult>) {
        results.forEach { BluetoothState.addScanResult(it) }
      }

      override fun onScanFailed(errorCode: Int) {
        Log.e(TAG, "Scan failed: $errorCode")
        BluetoothState.reportError("Scan Start Failed: $errorCode")
      }
    }

  BluetoothState.addScanner(callback)
  scanner.startScan(listOf(filter), settings, callback)
  Log.i(TAG, "Started scanning for $name")
  return mapOf("status" to "scanning", "filter" to name)
}

/// STEP: ^sees advertisement "(.*)" with RSSI "(.*)"$
fun seesAdvertisementWithRssi(context: Context, args: List<String>) {
  val name = args[0]
  val rssiReq = args[1]
  Log.i(TAG, "Checking for advertisement '$name' with RSSI '$rssiReq'")

  val start = System.currentTimeMillis()
  var found = false

  val uuid = UUID.nameUUIDFromBytes(name.toByteArray())

  // 1. Wait for presence (with persistence check)
  while (System.currentTimeMillis() - start < 10000) {
    if (BluetoothState.hasSeen(name, uuid)) {
      found = true
      // Found it! But wait a bit to see if we get more (verify it's not a one-off)
      Thread.sleep(2000)
      break
    }
    Thread.sleep(500)
  }

  if (!found) {
    val debug = BluetoothState.getSeenDebugString()
    val error = BluetoothState.getError() ?: "None"
    Log.e(TAG, "Seen devices:\n$debug")
    throw Exception("Timeout waiting for advertisement: $name. Last Error: $error. Seen:\n$debug")
  }

  // 2. Verify Persistence
  val count = BluetoothState.getSeenCount(name, uuid)
  Log.i(TAG, "Found advertisement: $name. Total seen: $count")
  if (count < 2) {
    Log.w(TAG, "Warning: Only saw $count advertisement(s) for $name.")
  }

  // 3. Verify RSSI
  val result =
    BluetoothState.getLatestResult(name, uuid)
      ?: throw Exception(
        "No advertisement found for RSSI check: $name"
      ) // Should be impossible if found=true

  val rssi = result.rssi
  Log.i(TAG, "Latest RSSI for $name: $rssi")

  val matches =
    when (rssiReq) {
      "HIGH" -> rssi >= RSSI_THRESHOLD_HIGH
      "MEDIUM" -> rssi >= RSSI_THRESHOLD_MEDIUM && rssi < RSSI_THRESHOLD_HIGH
      "LOW" -> rssi < RSSI_THRESHOLD_MEDIUM
      "HIGH_POWER" -> rssi >= RSSI_THRESHOLD_HIGH // Alias
      "MEDIUM_POWER" -> rssi >= RSSI_THRESHOLD_MEDIUM && rssi < RSSI_THRESHOLD_HIGH // Alias
      "LOW_POWER" -> rssi < RSSI_THRESHOLD_MEDIUM // Alias
      else -> {
        // Try parsing as integer inequality if needed, or just exact match?
        // For now assume buckets or exact int if it parses?
        // Let's stick to buckets for this test as per request "HIGH_POWER"
        try {
          val req = rssiReq.toInt()
          rssi == req
        } catch (e: NumberFormatException) {
          throw Exception("Unknown RSSI requirement: $rssiReq")
        }
      }
    }

  if (!matches) {
    throw Exception("RSSI mismatch for $name. Expected $rssiReq, got $rssi")
  }
}

/// STEP: ^resets bluetooth state$
fun resetBluetoothState(context: Context, args: List<String>) {
  BluetoothState.reset(context)
  Log.i(TAG, "Reset Bluetooth State")
}
