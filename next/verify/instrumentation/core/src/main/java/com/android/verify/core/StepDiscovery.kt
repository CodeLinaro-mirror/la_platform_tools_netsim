/*
 * Copyright 2026 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */
package com.android.verify.core

import android.util.Log
import dalvik.system.DexFile

fun StepRegistry.registerStepsFromScanning(
  scanPackage: String,
  nameFilter: (String) -> Boolean,
): Map<String, List<String>> {
  val inventory = mutableMapOf<String, List<String>>()
  val TAG = "StepDiscovery"
  try {
    val classLoader = this.context.classLoader
    // NOTE: This reflection-based approach to access pathList and dexElements is fragile,
    // as it depends on the private implementation details of BaseDexClassLoader.
    // This might break on future Android versions or on devices with custom class loaders.
    val pathListField = classLoader.javaClass.superclass.getDeclaredField("pathList")
    pathListField.isAccessible = true
    val pathList = pathListField.get(classLoader)
    val dexElementsField = pathList.javaClass.getDeclaredField("dexElements")
    dexElementsField.isAccessible = true
    val dexElements = dexElementsField.get(pathList) as Array<*>

    for (element in dexElements) {
      if (element == null) continue
      val dexFileField = element.javaClass.getDeclaredField("dexFile")
      dexFileField.isAccessible = true
      val dexFile = dexFileField.get(element) as DexFile? ?: continue
      val entries = dexFile.entries()
      while (entries.hasMoreElements()) {
        val className = entries.nextElement()
        if (className.startsWith(scanPackage) && nameFilter(className)) {
          try {
            Log.i(TAG, "Discovered step class: $className")
            val clazz = Class.forName(className)
            val steps = this.registerStepsFromClass(clazz)
            if (steps.isNotEmpty()) {
              inventory[className] = steps
            }
          } catch (e: Exception) {
            Log.e(TAG, "Failed to load discovered class: $className", e)
          }
        }
      }
    }
  } catch (e: Exception) {
    Log.e(TAG, "Failed to scan for steps", e)
  }
  return inventory
}
