/*
 * Copyright (C) 2024 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */

package android.test.wifi.nsd;

import android.content.Context;
import android.util.Log;
import androidx.test.filters.SmallTest;
import androidx.test.platform.app.InstrumentationRegistry;
import java.io.IOException;
import org.junit.BeforeClass;
import org.junit.Test;
import org.junit.runner.RunWith;
import org.junit.runners.JUnit4;

@RunWith(JUnit4.class)
public class NsdInstrumentationTest {
  private static final String TAG = NsdInstrumentationTest.class.getSimpleName();
  private static int deviceIdx;
  private static String testId;
  private static Context appContext;

  @BeforeClass
  public static void setup() {
    deviceIdx = Integer.valueOf(InstrumentationRegistry.getArguments().getString("position"));
    testId = InstrumentationRegistry.getArguments().getString("test_id");
    appContext = InstrumentationRegistry.getInstrumentation().getTargetContext();
  }

  @Test
  @SmallTest
  public void testNsd() throws InterruptedException, IOException {
    NsdHelper nsdHelper = new NsdHelper(appContext, testId);

    long startTime = System.currentTimeMillis();

    if (deviceIdx == 0) {
      // server mode
      nsdHelper.serviceTest();
    } else {
      nsdHelper.discoverTest();
    }
    // TODO: After adding the connection to end the tests on two device at the same time, execute
    // the tests repeatedly to make sure advertisement and discovery are stopped correctly.
    long duration = System.currentTimeMillis() - startTime;
    Log.d(TAG, "Duration: " + duration + " milliseconds");
  }
}
