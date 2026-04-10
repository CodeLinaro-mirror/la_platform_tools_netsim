/*
 * Copyright 2026 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */

package com.android.verify.vbs

import android.content.Context
import android.util.Log
import java.io.IOException
import java.net.ServerSocket
import java.net.Socket
import kotlin.concurrent.thread

private const val TAG = "TcpEchoServer"

/// STEP: ^starts a TCP echo server on "(.*)"$
fun startTcpEchoServer(context: Context, args: List<String>): Any {
  val varName = args[0]
  val serverSocket = ServerSocket(0)
  val port = serverSocket.localPort

  thread {
    try {
      while (true) {
        val clientSocket = serverSocket.accept()
        thread { handleClient(clientSocket) }
      }
    } catch (e: IOException) {
      Log.e(TAG, "Server socket closed or error: ${e.message}")
    }
  }

  val ip = getWlanIp()
  Log.i(TAG, "Started TCP echo server on $ip:$port")
  return mapOf(varName to "$ip:$port")
}

private fun getWlanIp(): String {
  try {
    val interfaces = java.net.NetworkInterface.getNetworkInterfaces()
    while (interfaces.hasMoreElements()) {
      val networkInterface = interfaces.nextElement()
      if (networkInterface.name == "wlan0") {
        val addresses = networkInterface.inetAddresses
        while (addresses.hasMoreElements()) {
          val inetAddress = addresses.nextElement()
          if (!inetAddress.isLoopbackAddress && inetAddress is java.net.Inet4Address) {
            return inetAddress.hostAddress?.toString() ?: "10.0.2.15"
          }
        }
      }
    }
  } catch (e: Exception) {
    Log.e(TAG, "Failed to get wlan0 IP", e)
  }
  return "10.0.2.15"
}

private fun handleClient(socket: Socket) {
  try {
    socket.use {
      val input = it.getInputStream()
      val output = it.getOutputStream()
      val buffer = ByteArray(1024)
      var bytesRead: Int
      while (input.read(buffer).also { bytesRead = it } != -1) {
        output.write(buffer, 0, bytesRead)
      }
    }
  } catch (e: Exception) {
    Log.e(TAG, "Error handling client: ${e.message}")
  }
}
