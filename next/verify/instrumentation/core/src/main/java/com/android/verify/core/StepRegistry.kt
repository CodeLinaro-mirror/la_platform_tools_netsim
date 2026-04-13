/*
 * Copyright 2026 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */
package com.android.verify.core

import android.content.Context
import android.util.Log
import java.util.regex.Pattern

class QuitException : RuntimeException("Quit")

class StepRegistry(internal val context: Context) {
  private val TAG = "StepRegistry"
  private val steps = mutableListOf<Pair<Pattern, (List<String>) -> Any?>>()

  fun register(regex: String, action: (List<String>) -> Any?) {
    Log.i(TAG, "Registering step pattern: $regex")
    steps.add(Pattern.compile(regex) to action)
  }

  fun registerStepsFromClass(clazz: Class<*>): List<String> {
    val registeredRegexes = mutableListOf<String>()
    for (method in clazz.declaredMethods) {
      val annotation = method.getAnnotation(Step::class.java) ?: continue
      val regex = annotation.regex
      Log.i(TAG, "Registering annotated step: $regex -> ${method.name}")
      register(regex) { args ->
        val paramTypes = method.parameterTypes
        if (
          paramTypes.size == 2 &&
            paramTypes[0] == Context::class.java &&
            paramTypes[1] == List::class.java
        ) {
          method.invoke(null, context, args)
        } else if (paramTypes.size == 1 && paramTypes[0] == List::class.java) {
          method.invoke(null, args)
        } else {
          throw IllegalArgumentException("Unsupported method signature for step: ${method.name}")
        }
      }
      registeredRegexes.add(regex)
    }
    return registeredRegexes
  }

  fun getRegisteredRegexes(): List<String> {
    return steps.map { it.first.pattern() }
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
