#!/bin/bash
# A robust wrapper for testing the netsimd E2E runner.
# This script manages the lifecycle of netsimd and android emulators, ensuring
# proper daemon cleanup via bash traps and robust wifi network state verification
# prior to invoking the bazel runner-e2e test suite.

set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
WORKSPACE_DIR="$( cd "$DIR/.." && pwd )"

# Default configuration.
EMULATOR_1="@Pixel_6_API_33"
EMULATOR_2="@Pixel_6_API_33b"
NETSIM_PID=""
EMU1_PID=""
EMU2_PID=""

export ANDROID_HOME=$HOME/Android/Sdk
export PATH=$HOME/Android/Sdk/platform-tools:$HOME/Android/Sdk/emulator:$PATH

cleanup() {
  echo ""
  echo "================================================="
  echo "Cleaning up dangling processes from E2E suite..."
  echo "================================================="
  set +e

  # Kill locally tracked PIDs if they exist
  if [ -n "$NETSIM_PID" ]; then kill -TERM $NETSIM_PID 2>/dev/null || true; fi
  if [ -n "$EMU1_PID" ];   then kill -TERM $EMU1_PID 2>/dev/null || true; fi
  if [ -n "$EMU2_PID" ];   then kill -TERM $EMU2_PID 2>/dev/null || true; fi

  # Force-kill any lingering netsimd instances started by other means
  pkill -u $(whoami) -9 -f "netsim+/next/daemon/daemon" 2>/dev/null || true
  pkill -u $(whoami) -9 -f "qemu-system-x86_64.*Pixel_6_API_33" 2>/dev/null || true
  killall -9 netsimd 2>/dev/null || true

  echo "Cleanup complete."
}

# Ensure cleanup runs on exit or interrupt
trap cleanup EXIT INT TERM

echo "================================================="
echo "1. Building the netsimd binary via Bazel"
echo "================================================="
cd "$WORKSPACE_DIR"
bazel build @netsim//next/daemon:daemon

echo "================================================="
echo "2. Aborting any previous runs"
echo "================================================="
cleanup

echo "================================================="
echo "3. Starting Netsims daemon"
echo "================================================="
# We run the binary directly from bazel-bin for immediate execution
NETSIM_BIN="$(bazel info bazel-bin)/external/netsim+/next/daemon/daemon"
$NETSIM_BIN --no-shutdown -v --logtostderr > /tmp/netsimd_e2e.log 2>&1 &
NETSIM_PID=$!

echo "Waiting for netsim gRPC to bind..."
sleep 2
GRPC_PORT=$(grep "grpc.port" /run/user/$(id -u)/netsim.ini | cut -d= -f2 || echo "")
if [ -z "$GRPC_PORT" ]; then
    echo "ERROR: Failed to read grpc.port from netsim.ini"
    exit 1
fi
echo "netsimd is alive on 127.0.0.1:$GRPC_PORT"

echo "================================================="
echo "4. Launching Emulators ($EMULATOR_1, $EMULATOR_2)"
echo "================================================="
emulator $EMULATOR_1 -no-window -no-audio -no-snapshot-save &
EMU1_PID=$!
emulator $EMULATOR_2 -no-window -no-audio -no-snapshot-save &
EMU2_PID=$!

echo "Waiting for 2 emulator instances to attach via adb..."
until [ "$(adb devices | grep -c -w "device")" -ge 2 ]; do
  sleep 5
done

for dev in $(adb devices | grep -w "device" | awk '{print $1}'); do
  echo "Waiting for $dev boot initialization..."
  until [ "$(adb -s $dev shell getprop sys.boot_completed | tr -d '\r')" == "1" ]; do
    sleep 3
  done

  echo "$dev is booted. Waiting for background network layer..."
  until adb -s $dev shell ping -c 1 -W 1 10.0.2.2 > /dev/null 2>&1; do
    sleep 2
  done

  echo "$dev Network stack is alive. Forcing WiFi connection to AndroidWifi..."
  adb -s $dev shell cmd wifi connect-network AndroidWifi open > /dev/null 2>&1 || true

  echo "Waiting for $dev wlan0 to receive an IP address..."
  # Time out after 30 seconds if IP fails
  WAIT_CYCLES=0
  until adb -s $dev shell ip addr show wlan0 | grep -q "inet "; do
    sleep 2
    ((WAIT_CYCLES++))
    if [ $WAIT_CYCLES -gt 15 ]; then
        echo "ERROR: $dev wlan0 failed to associate or pull an IP!"
        exit 1
    fi
  done

  echo "Success: $dev is fully connected to Wi-Fi!"
  adb -s $dev shell ip addr show wlan0 | grep "inet "
done

echo "================================================="
echo "5. Running E2E Test Suite via Bazel"
echo "================================================="
bazel test \
  --test_env=ANDROID_HOME=$ANDROID_HOME \
  --test_env=PATH=$PATH \
  --strategy=TestRunner=local \
  --test_output=streamed \
  @netsim//next/verify/runner:runner-e2e

echo "Tests finished successfully!"
# Trap will trigger the cleanup of all tracked PIDs here.
