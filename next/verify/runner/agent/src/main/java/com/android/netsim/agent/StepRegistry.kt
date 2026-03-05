/*
 * Copyright 2026 The Android Open Source Project
 */
package com.android.netsim.agent

import android.content.Context
import android.util.Log
import java.util.regex.Pattern

class StepRegistry(private val context: Context) {
  private val TAG = "StepRegistry"
  private val steps = mutableListOf<Pair<Pattern, (List<String>) -> Any?>>()

  fun register(regex: String, action: (List<String>) -> Any?) {
    Log.i(TAG, "Registering step pattern: $regex")
    steps.add(Pattern.compile(regex) to action)
  }

  fun execute(command: String): Pair<Boolean, Any?> {
    for ((pattern, action) in steps) {
      val matcher = pattern.matcher(command)
      if (matcher.matches()) {
        val args = mutableListOf<String>()
        for (i in 1..matcher.groupCount()) {
          args.add(matcher.group(i) ?: "")
        }
        try {
          Log.i(TAG, "Executing step: $command")
          val result = action(args)
          return true to result
        } catch (e: QuitException) {
          throw e
        }
      }
    }
    Log.e(TAG, "No matching step found for: $command")
    return false to null
  }
}
