/*
 * Copyright (C) 2025 The Android Open Source Project
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

import android.content.Context
import android.util.Log
import java.util.regex.Pattern

class StepRegistry(private val context: Context) {
    private val TAG = "StepRegistry"
    private val steps = mutableListOf<Pair<Pattern, (List<String>) -> Unit>>()

    fun register(regex: String, action: (List<String>) -> Unit) {
        steps.add(Pattern.compile(regex) to action)
    }

    fun execute(command: String): Boolean {
        for ((pattern, action) in steps) {
            val matcher = pattern.matcher(command)
            if (matcher.matches()) {
                val args = mutableListOf<String>()
                for (i in 1..matcher.groupCount()) {
                    args.add(matcher.group(i) ?: "")
                }
                try {
                    Log.i(TAG, "Executing step: $command")
                    action(args)
                    return true
                } catch (e: Exception) {
                    Log.e(TAG, "Step failed: $command", e)
                    return false
                }
            }
        }
        Log.e(TAG, "No matching step found for: $command")
        return false
    }
}
