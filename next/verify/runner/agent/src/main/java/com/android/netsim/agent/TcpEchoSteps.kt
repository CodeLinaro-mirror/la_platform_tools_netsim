/*
 * Copyright 2026 The Android Open Source Project
 */
 
package com.android.netsim.agent

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
                thread {
                    handleClient(clientSocket)
                }
            }
        } catch (e: IOException) {
            Log.e(TAG, "Server socket closed or error: ${e.message}")
        }
    }

    Log.i(TAG, "Started TCP echo server on port $port")
    return mapOf(varName to "10.0.2.15:$port")
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
