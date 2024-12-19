#!/usr/bin/env python3
#
# Copyright 2024 - The Android Open Source Project
#
# Licensed under the Apache License, Version 2.0 (the',  help="License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an',  help="AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
import argparse
import csv
import datetime
import os
import platform
import subprocess
import time
import psutil

PLATFORM_SYSTEM = platform.system().lower()
PLATFORM_MACHINE = platform.machine()
TEST_DURATION = 300
CURRENT_PATH = os.path.dirname(os.path.abspath(__file__))
NETSIMD_BINARY = 'netsimd.exe' if PLATFORM_SYSTEM == 'windows' else 'netsimd'
EMULATOR_BINARY = 'emulator.exe' if PLATFORM_SYSTEM == 'windows' else 'emulator'
QEMU_SYSTEM_BINARY = (
    f'qemu-system-{PLATFORM_MACHINE}.exe'
    if PLATFORM_SYSTEM == 'windows'
    else f'qemu-system-{PLATFORM_MACHINE}'
)


def _get_cpu_usage():
  """Utility function for getting netsimd CPU usage"""
  # Perform cpu_percent collection using psutil
  netsimd_cpu_usage, qemu_cpu_usage = [], []
  for process in psutil.process_iter(['name', 'cpu_percent']):
    if process.info['name'] == NETSIMD_BINARY:
      netsimd_cpu_usage.append(process.info['cpu_percent'])
    elif process.info['name'] == QEMU_SYSTEM_BINARY:
      qemu_cpu_usage.append(process.info['cpu_percent'])

  # Check for unreachable cases
  if len(netsimd_cpu_usage) > 1:
    raise LookupError(f'Multiple {NETSIMD_BINARY} processes found')
  if len(netsimd_cpu_usage) == 0:
    raise LookupError(f'Process {NETSIMD_BINARY} not found')
  if len(qemu_cpu_usage) > 1:
    raise LookupError(f'Multiple {QEMU_SYSTEM_BINARY} processes found')
  if len(qemu_cpu_usage) == 0:
    raise LookupError(f'Process {QEMU_SYSTEM_BINARY} not found')
  return netsimd_cpu_usage[0], qemu_cpu_usage[0]


def _trace_cpu_usage(filename):
  """Utility function for tracing CPU usage and write into csv"""
  with open(filename, 'w', newline='') as csvfile:
    writer = csv.writer(csvfile)
    writer.writerow(['Timestamp', NETSIMD_BINARY, QEMU_SYSTEM_BINARY])
    first_time = True
    for _ in range(TEST_DURATION):
      try:
        netsimd_cpu_usage, qemu_cpu_usage = _get_cpu_usage()
      except LookupError as e:
        print(e)
        time.sleep(1)
        continue
      if first_time:
        first_time = False
        time.sleep(0.1)
        continue
      data = [time.time(), netsimd_cpu_usage, qemu_cpu_usage]
      print(f'Got {data}')
      writer.writerow(data)
      time.sleep(1)


def _launch_emulator(cmd):
  """Utility function for launching Emulator"""
  if PLATFORM_SYSTEM == 'windows':
    return subprocess.Popen(
        cmd,
        creationflags=subprocess.CREATE_NEW_PROCESS_GROUP,
    )
  else:
    return subprocess.Popen(cmd)


def _terminate_emulator(process):
  """Utility function for terminating Emulator"""
  try:
    if PLATFORM_SYSTEM == 'windows':
      import signal

      process.send_signal(signal.CTRL_BREAK_EVENT)
      process.wait()
    else:
      process.terminate()
  except OSError:
    print('Process already termianted')


def _collect_cpu_usage(avd: str, netsim_wifi: bool):
  """Utility function for running the CPU usage collection session"""
  # Setup cmd and filename to trace
  time_now = datetime.datetime.now().strftime('%Y-%m-%d-%H-%M-%S')
  cmd = [f'{CURRENT_PATH}/{EMULATOR_BINARY}', '-avd', avd, '-wipe-data']
  filename = (
      f'netsimd_cpu_usage_{PLATFORM_SYSTEM}_{PLATFORM_MACHINE}_{time_now}.csv'
  )
  if netsim_wifi:
    cmd.extend(['-feature', 'WiFiPacketStream'])
    filename = f'netsimd_cpu_usage_{PLATFORM_SYSTEM}_{PLATFORM_MACHINE}_WiFiPacketStream_{time_now}.csv'

  # Launch emulator
  process = _launch_emulator(cmd)

  # Enough time for Emulator to boot
  time.sleep(10)

  # Trace CPU usage
  _trace_cpu_usage(filename)

  # Terminate Emulator Process
  _terminate_emulator(process)


def main():
  # Check if ANDROID_SDK_ROOT env is defined
  if 'ANDROID_SDK_ROOT' not in os.environ:
    print('Please set ANDROID_SDK_ROOT')
    return

  # Check if Emulator Binary exists
  emulator_path = f'{CURRENT_PATH}/{EMULATOR_BINARY}'
  if not os.path.isfile(emulator_path):
    print(
        f"Can't find {emulator_path}. Please place the file with the binaries"
        ' before executing.'
    )
    return

  # Set avd provided by the user
  parser = argparse.ArgumentParser()
  parser.add_argument('avd', help='The AVD to use', type=str)
  args = parser.parse_args()

  # Collect CPU usage without netsim WiFi
  _collect_cpu_usage(args.avd, False)

  # Enough time for Emulator to terminate
  time.sleep(10)

  # Collect CPU usage with netsim WiFi
  _collect_cpu_usage(args.avd, True)

  print('CPU Usage Completed!')


if __name__ == '__main__':
  main()
