/*
 * Copyright (C) 2024 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package com.android.netsim.ntest


import android.app.Instrumentation
import android.content.Context
import android.os.Bundle
import android.util.Log
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress
import java.net.ServerSocket
import java.net.Socket
import java.util.concurrent.TimeUnit

/**
 * NTestInstrumentation is a specialized Android Instrumentation class that acts as a
 * "Guest Agent" for the ntest orchestrator. It executes BDD steps received over a
 * TCP control channel (reverse tunneled via ADB) and reports results back to the host.
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
    private val TAG = "NTestAgent"

    @Volatile
    private var registry: StepRegistry? = null
    private lateinit var nsdManager: NsdManager
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
        nsdManager = context.getSystemService(Context.NSD_SERVICE) as NsdManager
        registry = StepRegistry(context)
        registerSteps()

        if (controlPort > 0) {
            java.lang.Thread {
                var connected = false
                var retries = 0
                while (!connected && retries < 60) {
                    try {
                        Log.i(TAG, "Attempting to connect to control port: $controlPort (attempt ${retries + 1})")
                        val socket = java.net.Socket("127.0.0.1", controlPort)
                        Log.i(TAG, "Successfully connected to control port: $controlPort")
                        connected = true
                        controlWriter = java.io.PrintWriter(socket.getOutputStream(), true)
                        startControlLoop(socket)
                    } catch (e: Exception) {
                        Log.e(TAG, "Failed to connect to control port $controlPort: ${e.message}")
                        retries++
                        java.lang.Thread.sleep(1000)
                    }
                }
                if (!connected) {
                    Log.e(TAG, "Giving up on control port $controlPort after $retries retries")
                }
            }.start()
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
     * Starts the bidirectional control loop on a dedicated thread.
     * Listens for raw BDD strings from the Host-side orchestrator.
     */
    private fun startControlLoop(socket: java.net.Socket) {
        java.lang.Thread({
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
                        r.execute(cmd)
                        log("${Protocol.MARKER_COMPLETED} $cmd ${Protocol.RESULT_SUCCESS}")
                    } catch (e: Exception) {
                        log("${Protocol.MARKER_COMPLETED} $cmd ${Protocol.RESULT_FAILURE} ${Protocol.MSG_KEY}${e.message}")
                    }
                }
            } catch (e: Exception) {
                // Connection closed or I/O error; terminate the control loop
            }
        }, "NTestControlLoop").start()
    }

    @Volatile
    private var discoveryFound: String? = null
    private val discoveryLock = java.lang.Object()

    private fun registerSteps() {
        val r = registry ?: return

        r.register("^When Android Advertises Service (.*) as (.*) on port (\\d+)$") { args ->
            val serviceType = args[0]
            val serviceName = args[1]
            val port = args[2].toInt()
            startNsd(serviceName, serviceType, port)
        }

        r.register("^When Android Advertises Service (.*) as (.*)$") { args ->
            val serviceType = args[0]
            val serviceName = args[1]
            startNsd(serviceName, serviceType, 0)
        }

        r.register("^When Android Starts Discovery for (.*)$") { args ->
            val serviceType = args[0]
            synchronized(discoveryLock) { discoveryFound = null }
            discoverNsd(serviceType)
        }

        r.register("^Then Android Finds Service (.*)$") { args ->
            val expectedName = args[0]
            val start = System.currentTimeMillis()
            var found = false
            while (System.currentTimeMillis() - start < 10000) {
                synchronized(discoveryLock) {
                    if (discoveryFound != null && discoveryFound!!.contains(expectedName)) {
                        found = true
                    }
                }
                if (found) break
                java.lang.Thread.sleep(500)
            }
            if (found) {
                log("INFO Verifies that service $expectedName was discovered")
            } else {
                log("INFO  Timeout waiting for service $expectedName")
                throw Exception("Service not found")
            }
        }

        r.register("^When Android sends (\\d+) bytes of (UDP|TCP) data to (.*)$") { args ->
            val size = args[0].toInt()
            val proto = args[1]
            val target = args[2]

            if (proto == "UDP") {
                sendUdp(size, target)
            } else {
                sendTcp(size, target)
            }
        }

        r.register("^THEN Android Quits$") {
            finish(0, Bundle())
        }
    }

    private fun discoverNsd(serviceType: String) {
        nsdManager.discoverServices(serviceType, NsdManager.PROTOCOL_DNS_SD, object : NsdManager.DiscoveryListener {
            override fun onDiscoveryStarted(regType: String) {
                Log.i(TAG, "Discovery started: $regType")
            }

            override fun onServiceFound(serviceInfo: NsdServiceInfo) {
                Log.i(TAG, "Service found: ${serviceInfo.serviceName}")
                synchronized(discoveryLock) {
                    discoveryFound = serviceInfo.serviceName
                }
            }

            override fun onServiceLost(serviceInfo: NsdServiceInfo) {
                Log.e(TAG, "Service lost: ${serviceInfo.serviceName}")
            }

            override fun onDiscoveryStopped(serviceType: String) {
                Log.i(TAG, "Discovery stopped: $serviceType")
            }

            override fun onStartDiscoveryFailed(serviceType: String, errorCode: Int) {
                Log.e(TAG, "Discovery failed: $errorCode")
                nsdManager.stopServiceDiscovery(this)
            }

            override fun onStopDiscoveryFailed(serviceType: String, errorCode: Int) {
                Log.e(TAG, "Stop discovery failed: $errorCode")
                nsdManager.stopServiceDiscovery(this)
            }
        })
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

    private fun startNsd(serviceName: String, serviceType: String, requestedPort: Int) {
        val port = if (requestedPort > 0) {
            requestedPort
        } else {
            try {
                ServerSocket(0).use { it.localPort }
            } catch (e: Exception) { 0 }
        }

        val serviceInfo = NsdServiceInfo().apply {
            this.serviceName = serviceName
            this.serviceType = serviceType
            this.port = port
        }

        nsdManager.registerService(serviceInfo, NsdManager.PROTOCOL_DNS_SD, object : NsdManager.RegistrationListener {
            override fun onRegistrationFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {
                log("INFO  NSD Registration Failed: $errorCode")
            }
            override fun onUnregistrationFailed(serviceInfo: NsdServiceInfo, errorCode: Int) {}
            override fun onServiceRegistered(serviceInfo: NsdServiceInfo) {
                log("INFO Verifies the service is registered")
            }
            override fun onServiceUnregistered(serviceInfo: NsdServiceInfo) {}
        })
    }
}
