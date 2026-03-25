---
description: How to run the Netsim e2e integration test suite using Bazel and Android Emulators manually.
---

# Netsim E2E Runner Workflow

When you need to run the `verify/runner:runner-e2e` tests, you must bring up the required daemons and ensure Bazel has the correct environment flags to communicate with the host's ADB daemon, bypassing the strict `linux-sandbox`.

## 1. Clean up stale processes
Kill lingering daemon and emulator processes from previous runs:
```bash
killall -9 netsimd qemu-system-x86_64 emulator || true
```

## 2. Launch netsimd
Launch the netsim daemon in the background to serve the test:
```bash
# turbo
bazel run @netsim//next/daemon:daemon -- --no-shutdown -v --logtostderr &
```

## 3. Identify and Run Emulators
Find the available AVDs:
```bash
emulator -list-avds
```
Then, pick the AVDs (e.g., `Pixel_6_API_33` and `Pixel_6_API_33b`) and launch them in the background. The test requires at least one attached device:
```bash
# turbo
emulator @Pixel_6_API_33 -no-window -no-audio &
# And potentially a second one if the scenario requires P2P networking:
# emulator @Pixel_6_API_33b -no-window -no-audio &
```
Wait until `adb devices` shows the device(s) as attached.

## 4. Run the BDD Test Suite
Because the `ntest run` command relies on ADB, and Bazel limits environment variables and network access (sandbox), you must **explicitly pass both PATH and ANDROID_HOME** via `--test_env` AND force a local execution strategy (`--strategy=TestRunner=local` or `--test_strategy=local`) so the test can connect to your host's ADB server on port `5037`.

Execute the test:
```bash
# turbo
bazel test \
  --test_env=ANDROID_HOME=$HOME/Android/Sdk \
  --test_env=PATH=$HOME/Android/Sdk/platform-tools:$HOME/Android/Sdk/emulator:$PATH \
  --strategy=TestRunner=local \
  --test_output=streamed \
  @netsim//next/verify/runner:runner-e2e
```
