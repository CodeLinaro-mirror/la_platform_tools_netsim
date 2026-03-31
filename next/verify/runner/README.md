# ntest (Netsim Network Tester)

`ntest` is a descriptive BDD-style orchestration tool designed to validate complex networking scenarios between the Host and one or more Guest devices (Android Emulators).

## Key Features

- **Multi-Device Orchestration**: Automatically discovers available emulators via ADB and manages the lifecycle of the test environment.
- **Narrative BDD Output**: Provides a professional, aligned console output using `GIVEN / WHEN / THEN / INFO` tags for clear auditability of test steps.
- **Kotlin Agent Integration**: Pairs with a companion `ntest-agent` APK running as a service on Android guests, enabling high-fidelity mobile network testing.
- **iperf3 Benchmarking**: Built-in support for high-precision throughput measurements with industry-standard `iperf3` formatting, including interval samples and summaries.
- **Variable Resolution**: Supports dynamic template placeholders like `{port}`, `{target}`, and `{gateway_target}` that resolve automatically during execution.

## Prerequisites

To build and run the complete E2E test suite (including the Android Agent), you need:
- **Android Studio / SDK**: Installed and configured.
- **Environment Variables**: `ANDROID_HOME` (or `ANDROID_SDK_ROOT`) exported and pointing to your SDK path (e.g. `export ANDROID_HOME=$HOME/Android/Sdk`).
- **Build Tools**: The Android `build-tools` (which include `aapt2`) must be installed. It is highly recommended to add them to your `PATH` so Bazel's `rules_android` can locate them.

## Quick Start

### Building

Build the orchestrator and the E2E wrapper:

```bash
bazel build @netsim//next/testing/ntest:ntest
bazel build @netsim//next/testing/ntest:ntest-e2e
```

### Running Tests

The primary command for integration testing is `run`. It orchestrates the host-side server and the guest-side clients.

```bash
# Run all scenarios on a connected device
./ntest run --apk-path ntest-agent.apk --netsim-path netsimd

# List available scenarios (Dry-Run Mode)
./ntest scenarios
```

### E2E Wrapper (Production)

For automated environments, use the `ntest-e2e` shell wrapper which resolves paths and dependencies automatically:

```bash
bazel run @netsim//next/testing/ntest:ntest-e2e
```

## CLI Usage

```text
Options:
  --android-home <PATH>   Path to the Android SDK root, platform-tools, or adb binary
  --apk-path <PATH>       Path to the ntest-agent APK
  --netsim-path <PATH>    Path to netsim binary
  --netsim-args <ARGS>    Arguments to pass to netsim
  --gateway-ip <IP>       Gateway IP to connect to (defaults to 10.0.2.2)
```

### Discovery Logic

The orchestrator intelligently locates `adb` by checking:
1. The provided `--android-home` flag (checking both the path and `path/platform-tools/adb`).
2. The `ANDROID_HOME`, `ANDROID_SDK_ROOT`, or `ANDROID_SDK_HOME` environment variables.
3. The system `PATH`.

## BDD Narrative Format

`ntest` produces a vertically aligned narrative that describes the interactions between actors:

```text
SCENARIO: Basic TCP Echo
GIVEN  @adb          Has 1 or more attached devices
WHEN   @Host         Starts a TCP echo server on "port"
INFO   @Host         Echo server listening on port=34829
AND    @Small_Phone  Sends 1KB TCP to 10.0.2.2:34829
INFO   @Small_Phone  Verifies the TCP echo (1024 bytes)
THEN   @Host         Receives 1KB TCP data
```

### Understanding INFO vs. BDD Steps

`ntest` distinguishes between behavioral intent and runtime telemetry:

- **BDD Steps (`GIVEN`, `WHEN`, `THEN`)**: These define the high-level behavioral "What".
    - `GIVEN` sets the environment (e.g., "An active server on 'port'").
    - `WHEN` triggers an action (e.g., "Sends 1KB TCP").
    - `THEN` asserts an outcome (e.g., "Receives data").
- **Observational Lines (`INFO`)**: These provide the runtime "How".
    - **Telemetry**: Dynamic values like kernel-assigned ports (e.g., `port=34829`).
    - **Side-Effects**: Progress logs like "Installing APK" or "Launching Agent".
    - **Actor Feedback**: Self-reporting from the Android guest to confirm internal checks.

**Dry-Run Mode**: When running `./ntest scenarios`, the `INFO` lines are automatically suppressed to provide a clean view of the behavioral design.

### Actor Identification

- **`@Host`**: The Linux host machine running the orchestrator.
- **`@adb`**: The ADB Bridge (used for environment setup/discovery logs).
- **`@AVD#N`**: Generic placeholders that resolve to specific devices (e.g., `@Small_Phone`).

## Benchmarking

Scenarios like `Gateway Performance` utilize the `run_benchmark` step to produce high-resolution performance metrics:

```text
SCENARIO: Gateway Performance
GIVEN  @adb          Has 1 or more attached devices
WHEN   @Host         Starts an echo server on the gateway on "port"
INFO   @Host         Echo server listening on port=44877
THEN   @Small_Phone  Measures performance with 10 samples of 1MB TCP to 10.0.2.2:44877
INFO   @Small_Phone  [ ID] Interval           Transfer     Bitrate
INFO   @Small_Phone  [  5]  0.00- 1.02 sec    1.00 MBytes    8.22 Mbits/sec
INFO   @Small_Phone  [  5]  1.02- 1.97 sec    1.00 MBytes    8.86 Mbits/sec
...
INFO   @Small_Phone  - - - - - - - - - - - - - - - - - - - - - - - - -
INFO   @Small_Phone  [  5]  0.00- 9.42 sec   10.00 MBytes    8.92 Mbits/sec
```

## Internal Architecture

The orchestrator implements a **Control Plane** and a **Data Plane**:

1.  **Control Plane (Host-to-Guest)**: Communicates with the Kotlin agent via ADB port forwarding and a lightweight command protocol.
2.  **Data Plane (Guest-to-Host)**: Actual test traffic (TCP/UDP) flows between the Guest agent and the Host echo server through the Netsim medium (Slirp or TAP).
3.  **Feedback Channel**: A persistent TCP connection from the Guest back to the Host that streams live BDD step status and logs.
