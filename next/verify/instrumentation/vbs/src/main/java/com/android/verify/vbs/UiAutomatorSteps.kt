/*
 * Copyright 2026 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */
package com.android.verify.vbs

import android.content.Context
import android.net.wifi.WifiManager
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.uiautomator.By
import androidx.test.uiautomator.UiDevice
import androidx.test.uiautomator.Until
import com.android.verify.core.Step
import com.android.verify.core.logToHost

private const val TAG = "Verify:UiAutomator"
private const val DEFAULT_TIMEOUT = 5000L

private fun getDevice(): UiDevice =
  UiDevice.getInstance(InstrumentationRegistry.getInstrumentation())

@Step("clicks on element with text \"(.*)\"")
fun clickOnElementWithText(context: Context, args: List<String>) {
  val (text) = args
  context.logToHost("INFO Clicking on element with text: $text")

  val element =
    getDevice().wait(Until.findObject(By.text(text)), DEFAULT_TIMEOUT)
      ?: throw Exception("Element with text \"$text\" not found")
  element.click()
}

@Step("enters \"(.*)\" into field with text \"(.*)\"")
fun enterTextIntoFieldWithText(context: Context, args: List<String>) {
  val (textToEnter, fieldText) = args
  context.logToHost("INFO Entering \"$textToEnter\" into field with text: $fieldText")

  val element =
    getDevice().wait(Until.findObject(By.text(fieldText)), DEFAULT_TIMEOUT)
      ?: throw Exception("Field with text \"$fieldText\" not found")
  element.text = textToEnter
}

@Step("should see text \"(.*)\"")
fun shouldSeeText(context: Context, args: List<String>) {
  val text = args[0]
  val errorMsg = if (args.size > 1) args[1] else "Text \"$text\" not found"

  if (
    androidx.test.platform.app.InstrumentationRegistry.getArguments().getString("verbose") == "true"
  ) {
    context.logToHost("INFO Checking for text: $text")
  }

  if (!getDevice().wait(Until.hasObject(By.text(text)), DEFAULT_TIMEOUT)) {
    throw Exception(errorMsg)
  }
}

@Step("sets Wi-Fi to (enabled|disabled) via UI")
fun setWifiStateViaSettings(context: Context, args: List<String>) {
  val (expectedState) = args
  context.logToHost("INFO Setting Wi-Fi to $expectedState via UI")

  val device = getDevice()

  // Open settings
  context.startActivity(
    android.content.Intent(android.provider.Settings.ACTION_WIFI_SETTINGS).apply {
      addFlags(android.content.Intent.FLAG_ACTIVITY_NEW_TASK)
    }
  )

  // Wait for settings to open
  shouldSeeText(context, listOf("Wi-Fi", "Wi-Fi settings page not opened"))

  // Wait for idle to ensure UI is stable
  device.waitForIdle()

  // Find the switch element.
  val wifiSwitch =
    device.wait(Until.findObject(By.clazz("android.widget.Switch")), DEFAULT_TIMEOUT)
      ?: throw Exception("Wi-Fi switch not found in Settings")

  val shouldBeChecked = expectedState == "enabled"
  if (wifiSwitch.isChecked != shouldBeChecked) {
    context.logToHost("INFO Clicking switch to make it $expectedState")
    wifiSwitch.click()
  } else {
    context.logToHost("INFO Wi-Fi is already $expectedState")
  }

  // Go back
  device.pressBack()
}

@Step("Android Wi-Fi is disabled")
fun verifyWifiIsDisabled(context: Context, args: List<String>) {
  val wm = context.getSystemService(Context.WIFI_SERVICE) as WifiManager
  for (i in 0..10) {
    if (!wm.isWifiEnabled) {
      context.logToHost("INFO Wi-Fi is disabled as expected")
      return
    }
    Thread.sleep(500)
  }
  throw Exception("Wi-Fi is enabled, expected disabled")
}

@Step("Android Wi-Fi is enabled")
fun verifyWifiIsEnabled(context: Context, args: List<String>) {
  val wm = context.getSystemService(Context.WIFI_SERVICE) as WifiManager
  for (i in 0..10) {
    if (wm.isWifiEnabled) {
      context.logToHost("INFO Wi-Fi is enabled as expected")
      return
    }
    Thread.sleep(500)
  }
  throw Exception("Wi-Fi is disabled, expected enabled")
}
