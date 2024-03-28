# Example Mobly Test

This direcotry contains example Mobly tests.

The tests here works with Mobly in AOSP as well as from GitHub (https://github.com/google/mobly).

## Instructions
The Mobly tests generally assume android devices are already running on the host.
Prior to running the example tests please start two android virtual devices and verify they are connected to adb.

### To run using AOSP Mobly:
Ensure you are using Cuttlefish virtual device when launching with atest. AOSP Mobly support was developed with Cuttlefish only.
Running with Goldfish emulator isn't fully supported and may encounter errors such as apk install failures.

Simply invoke atest on the test module defined in Android.bp:
*  `atest ble-gatt-test`

### To run with standalone AOSP Mobly runner:

Use Mobly's local test runner script with the test module and Mobly YAML config file:
*  `tools/test/mobly_extensions/scripts/local_mobly_runner.py -m ble-gatt-test -c tools/netsim/testing/mobly/sample_config.yml`

Refer to the `local_mobly_runner.py` script or Mobly documentation for additional info about the runner.

### To run with Mobly on GitHub:

1. Clone the open source Mobly on GitHub (https://github.com/google/mobly)
2. Either place the example tests under your Mobly checkout or otherwise handle importing / installing Mobly manually.
3. Ensure mobly bundled snippets (https://github.com/google/mobly-bundled-snippets) is installed on the devices.
4. Execute Mobly test with python and use the Mobly YAML config file. Example command:
    * `python3 ./tests/betosim/ble_gatt_test.py -c ./tests/betosim/sample_config.yml`

