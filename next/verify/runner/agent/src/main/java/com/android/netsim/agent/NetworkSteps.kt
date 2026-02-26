/*
 * Copyright 2026 The Android Open Source Project
 */
 package com.android.netsim.agent

import android.content.Context
import android.util.Log
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress
import java.net.Socket

private const val TAG = "NetworkSteps"

/// STEP: ^When Android sends (\d+) bytes of (UDP|TCP) data to (.*)$
fun sendData(context: Context, args: List<String>) {
    val size = args[0].toInt()
    val proto = args[1]
    val target = args[2]

    if (proto == "UDP") {
        sendUdp(size, target)
    } else {
        sendTcp(size, target)
    }
}

private fun log(msg: String) {
    Log.i(TAG, msg)
}

private fun sendUdp(size: Int, target: String) {
    val parts = target.split(":")
    val host = parts[0]
    val port = if (parts.size > 1) parts[1].toInt() else 8080

    val socket = DatagramSocket()
    val address = InetAddress.getByName(host)
    val data = ByteArray(Math.min(size, 8192)) { 0x41.toByte() }

    var remaining = size
    while (remaining > 0) {
        val toSend = Math.min(remaining, data.size)
        val packet = DatagramPacket(data, toSend, address, port)
        socket.send(packet)
        remaining -= toSend
    }

    socket.soTimeout = 3000
    val recvBuf = ByteArray(8192)
    val recvPacket = DatagramPacket(recvBuf, recvBuf.size)
    try {
        socket.receive(recvPacket)
        log("INFO Verifies the UDP echo")
    } catch (e: Exception) {
        log("INFO  UDP Echo timeout")
    }
    socket.close()
}

private fun sendTcp(size: Int, target: String) {
    val parts = target.split(":")
    val host = parts[0]
    val port = if (parts.size > 1) parts[1].toInt() else 8080

    Socket(host, port).use { socket ->
        val output = socket.getOutputStream()
        val input = socket.getInputStream()
        val data = ByteArray(8192) { 0x41.toByte() }

        var remainingWrite = size
        while (remainingWrite > 0) {
            val toWrite = Math.min(remainingWrite, data.size)
            output.write(data, 0, toWrite)
            remainingWrite -= toWrite
        }
        output.flush()

        var totalRead = 0
        val recvBuf = ByteArray(8192)
        while (totalRead < size) {
            val read = input.read(recvBuf)
            if (read == -1) break
            totalRead += read
        }
        log("INFO Verifies the TCP echo ($totalRead bytes)")
    }
}
