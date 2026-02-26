/*
 * Copyright 2026 The Android Open Source Project
 */
 package com.android.netsim.agent

import android.content.Context
import android.util.Log

private const val WIFI_TAG = "WifiSteps"

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


