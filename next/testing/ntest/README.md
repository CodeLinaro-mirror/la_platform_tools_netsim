# ntest (Network Tester)

`ntest` is a network verification tool to validate TAP and Slirp guest
networking between emulator and a host. It compiles into a host and android
binary that acts as a test-runner and test-server on the host and test-client on
android.

Note: because of guest-host networking, the guest connects to host.

The test runner can launch netsimd, emulator, install the android binary on the
emulator, and run the test scenarios.

The main scenarios are:

- **TCP Echo:** (Basic) Connects via TCP, sends a payload, and verifies the
  echoed response matches.
- **UDP Echo:** (Basic) Connects via UDP, sends a payload, and verifies the
  echoed response.
- **Gateway Performance:** (Integration) Connects to the Host via its Gateway
  address and performs a large data transfer (500MB) to verify stability and
  throughput. _Note: "Gateway" here refers to the Host's address reachable from
  the Guest: `10.0.2.2` (Slirp User Networking) or the assigned TAP interface
  address (Bridged/Routed)._

### Planned Scenarios

- **mDNS Discovery:** Verifies multicast routing and service discovery
  (guest-to-host).
- **Concurrent Connections:** Stress tests the link with multiple simultaneous
  clients.
- **Bidirectional Transfer:** Verifies full-duplex performance by sending data
  both ways simultaneously.

## Overview

`ntest` operates in three primary modes:

1.  **Test Runner:** Orchestrates the Client and Server to run complex scenarios
    (e.g., local loopback tests or Android-to-Host integration tests).
2.  **Server (Reflector):** Binds to ports and echos data back. Run on the host.
3.  **Client (Tester):** Connects to a target, sends data, and verifies the
    echo. Usually run on the guest

## Building

To build the `ntest` binary:

### Host (Linux/Mac)

```bash
bazel build @netsim//next/testing/ntest:ntest
```

### Android (aarch64)

**Option 1: Script (Recommended)**

Prerequisites: `ANDROID_SDK_HOME` must be set.

```bash
export ANDROID_SDK_HOME=/path/to/android-sdk
./build_android.sh
```

The output will be at
`../../target/aarch64-linux-android/release/ntest_android`.

**Option 2: Bazel (Advanced)**

_Note: Requires local NDK restrictions/rules configuration._

```bash
bazel build --config=android @netsim//next/testing/ntest:ntest
```

The output will be in `bazel-bin/tools/netsim/next/testing/ntest/ntest`.

## Usage

The primary way to use `ntest` is via the **Local** or **Android** subcommands.

```bash
# List available scenarios
./ntest scenarios
```

### Local Mode (Self-Test)

Runs both client and server locally on the host machine using loopback. Useful
for developing test logic.

```bash
./ntest local
```

### Android Mode (Integration Test)

Runs the server on the Host and the client on a connected Android device (via
ADB).

```bash
# Run on an existing emulator/device (or launch if none found)
./ntest android --android-bin <path_to_android_binary>
```

**Note:** For Android mode, you need to provide the path to the `ntest` binary
cross-compiled for Android (usually aarch64). See [Building](#building) section
for instructions.

Then pass the resulting binary:

```bash
./ntest android --android-bin bazel-bin/tools/netsim/next/testing/ntest/ntest
```

#### Android Mode Workflow

In `android` mode, the Test Runner manages the entire lifecycle:

1.  **Netsim Launch:** If `--netsim-bin` is provided, the runner launches that
    specific Netsim instance.
2.  **Emulator Check:** It checks for connected ADB devices.
    - If a device is found, it uses it.
    - If no device is found, it launches a new emulator instance (using
      available AVDs).
3.  **Setup:** Pushes the `ntest` binary to `/data/local/tmp/ntest` and waits
    for network connectivity (`ping 10.0.2.2`).
4.  **Execution:** Runs the test scenarios, coordinating the Host Server and
    Android Client.

### Testing Custom Netsim Versions

You can run tests against a specific `netsim` binary or pass custom arguments.

```bash
# Use a custom netsim binary
./ntest android --netsim-bin <path_to_netsim>

# Pass arguments to netsim (passed to netsimd or emulator -netsim-args)
./ntest android --netsim-args "--packet-capture"
```

### Verifying TAP Configuration

`ntest` supports verifying TAP interface availability on the Host.

In **Local Mode**, this verifies that the Host can bind to and reach the TAP
interface's Gateway IP (e.g. `192.168.96.1`), confirming the interface is up and
configured.

```bash
# Verify Cuttlefish TAP pool (implies 192.168.96.1 gateway)
./ntest local --wifi-cvd-tap

# Verify specific TAP interface
./ntest local --wifi-tap tap0 --gateway-ip 192.168.1.1
```

To use TAP in **Android Mode**, pass the arguments to `netsim`:

```bash
./ntest android --netsim-args "--wifi-cvd-tap"
```

## Support Modes

These modes are used internally by the Test Runner or for manual debugging.

### Server Mode (Reflector)

Runs a TCP/UDP echo server.

```bash
# Listen on a specific port
./ntest server --port 8080

# Listen on a random port (prints port to stdout)
./ntest server --port 0
```

### Client Mode (Tester)

Runs a single test against a target.

```bash
# TCP Echo Test
./ntest client --proto tcp --target 127.0.0.1:8080 --payload-size 1024

# UDP Echo Test
./ntest client --proto udp --target 127.0.0.1:8080 --payload-size 512
```

## Architecture

The system relies on a **Reflector Pattern**:

- **Local Mode:**
  - **Controller:** Spawns a local Server process.
  - **Client:** Spawns a local Client process connecting to `127.0.0.1`.
  - **Result:** Immediate verification of test logic.

- **Android Mode:**
  - **Controller (Host):** running `ntest android ...`.
  - **Server (Host):** Spawns `ntest server` listening on `0.0.0.0` (accessible
    from Guest).
  - **Client (Guest):** Pushes and executes `ntest client` on Android via ADB,
    connecting to `10.0.2.2` (Host Alias).

This "Native BDD" approach defines tests in purely Rust code
(`src/tests/scenarios.rs`), giving us compile-time safety and zero parsing
overhead.
