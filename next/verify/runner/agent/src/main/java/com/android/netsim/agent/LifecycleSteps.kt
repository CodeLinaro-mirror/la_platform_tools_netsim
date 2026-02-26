/*
 * Copyright 2026 The Android Open Source Project
 */
package com.android.netsim.agent

import android.app.Instrumentation
import android.content.Context
import android.os.Bundle

/// STEP: ^THEN Android Quits$
fun quit(context: Context, args: List<String>) {
    // We need access to Instrumentation to finish.
    // If context is Instrumentation, we good.
    // Or we throw a specialized exception that NTestInstrumentation catches to finish?
    // Or we use a Singleton?
    throw QuitException()
}

class QuitException : RuntimeException("Quit")
