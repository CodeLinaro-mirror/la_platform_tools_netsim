# Copyright 2024 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

import uuid
from mobly import base_test
from mobly import test_runner
from mobly import utils
from mobly.asserts import assert_not_in
from mobly.controllers import android_device


class MultiDeviceInstrumentationTest(base_test.BaseTestClass):

  def setup_class(self):
    self.devices = self.register_controller(android_device)

  def test_in_parallel(self):
    test_id = str(uuid.uuid1())[:8]

    def run_instrument_cmd(device_idx, device):
      result = device.adb.shell((
          f'am instrument -w -e position {device_idx} -e test_id {test_id} '
          + 'android.test.wifi.nsd/androidx.test.runner.AndroidJUnitRunner'
      ))
      assert_not_in('FAIL', result.decode(), f'Failed in device {device_idx}')

    utils.concurrent_exec(
        run_instrument_cmd,
        [*enumerate(self.devices)],
        max_workers=2,
        raise_on_exception=True,
    )


if __name__ == '__main__':
  if '--' in sys.argv:
    index = sys.argv.index('--')
    sys.argv = sys.argv[:1] + sys.argv[index + 1 :]

  test_runner.main()
