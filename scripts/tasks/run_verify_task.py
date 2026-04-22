#!/usr/bin/env python3
# Copyright 2026 - The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

import logging
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time
from tasks.task import Task
from utils import AOSP_ROOT, EMULATOR_ARTIFACT_PATH, configure_android_sdk, run


class RunVerifyTask(Task):
  """Task to run verify integration tests with emulators."""

  def __init__(self, args, env):
    super().__init__("RunVerify")
    self.args = args
    self.env = env
    self.out = Path(args.out_dir)

  def do_run(self):
    # Create AVDs using avdmanager CLI

    self.out.mkdir(parents=True, exist_ok=True)
    sdk_path, is_prebuilt = configure_android_sdk()
    sdk_root = Path(sdk_path)

    if is_prebuilt:
      # Use prebuilts layout in repo
      system_images_dir = sdk_root.parents[1]
      linux_sdk_dir = system_images_dir / "linux"
      avdmanager_bin = (
          linux_sdk_dir / "cmdline-tools" / "latest" / "bin" / "avdmanager"
      )
    else:
      # Use standard layout
      avdmanager_bin = (
          sdk_root / "cmdline-tools" / "latest" / "bin" / "avdmanager"
      )

    if self.args.buildbot:
      emulator_dir = EMULATOR_ARTIFACT_PATH / "emulator"
    else:
      emulator_dir = self.out / "distribution" / "emulator"

    emulator_bin = emulator_dir / "emulator"

    if not avdmanager_bin.exists():
      logging.error(f"avdmanager not found at {avdmanager_bin}")
      return False
    if not emulator_bin.exists():
      logging.error(f"emulator binary not found at {emulator_bin}")
      return False

    configs = [
        {
            "api": "33",
            "tag.id": "google_apis",
            "abi": "x86_64",
            "AvdId": f"CI-AVD-{i}",
        }
        for i in range(3)
    ]

    avds = []
    emu_procs = []
    netsimd_proc = None
    netsimd_log = None
    emu_logs = []
    try:
      logging.info("Creating AVDs...")
      for cfg in configs:
        name = cfg["AvdId"]
        package = (
            f"system-images;android-{cfg['api']};{cfg['tag.id']};{cfg['abi']}"
        )

        if is_prebuilt:
          image_dir = (
              sdk_root.parents[1]
              / "linux"
              / "system-images"
              / f"android-{cfg['api']}"
              / cfg["tag.id"]
              / cfg["abi"]
          )
          if image_dir.exists():
            blocksize = 8192
            # Uncompress gzipped system images
            for file_gz in image_dir.glob("*.gz"):
              file = file_gz.parent / file_gz.stem
              if not file.exists() or os.path.getmtime(
                  file
              ) <= os.path.getmtime(file_gz):
                logging.info(f"Extracting {file_gz}...")
                import gzip

                with gzip.open(file_gz, "rb") as file_gz_in:
                  with open(file, "wb") as file_gz_out:
                    shutil.copyfileobj(file_gz_in, file_gz_out, blocksize)

          # Copy hardware-properties.ini from emulator to platform directory as it is needed for avdmanager to work
          hw_props_src = emulator_dir / "lib" / "hardware-properties.ini"
          hw_props_dest = (
              sdk_root.parents[1]
              / "linux"
              / "emulator"
              / "lib"
              / "hardware-properties.ini"
          )
          if hw_props_src.exists() and not hw_props_dest.exists():
            logging.info(
                f"Copying {hw_props_src} to {hw_props_dest} for avdmanager..."
            )
            try:
              hw_props_dest.parent.mkdir(parents=True, exist_ok=True)
              shutil.copy(hw_props_src, hw_props_dest)
            except Exception as e:
              logging.warning(
                  f"Failed to copy hardware-properties.ini: {e}. avdmanager may"
                  " fail."
              )

        logging.info(f"Creating AVD {name} with package {package}...")

        # Run avdmanager create avd non-interactively
        process = subprocess.run(
            [
                str(avdmanager_bin),
                "create",
                "avd",
                "--name",
                name,
                "--package",
                package,
                "--force",
            ],
            input=b"no\n",  # Answer "no" to custom hardware profile prompt
            capture_output=True,
            check=False,
        )
        if process.returncode != 0:
          logging.error(
              f"Failed to create AVD {name}: {process.stderr.decode('utf-8')}"
          )
          return False
        avds.append(name)

      # Start netsimd
      netsimd_bin = AOSP_ROOT / "bazel-bin/external/netsim+/next/daemon/daemon"
      if not netsimd_bin.exists():
        logging.error(f"netsimd binary not found at {netsimd_bin}")
        return False
      netsimd_log_path = self.out / "netsimd_e2e.log"
      logging.info(f"Logging netsimd to {netsimd_log_path}...")
      netsimd_log = open(netsimd_log_path, "w")
      netsimd_proc = subprocess.Popen(
          [str(netsimd_bin), "--no-shutdown", "-v", "--logtostderr"],
          stderr=netsimd_log,
          stdout=netsimd_log,
      )

      # Start emulators
      logging.info("Starting emulators...")
      for i, avd_name in enumerate(avds):
        emu_log_path = self.out / f"emulator_{avd_name}_e2e.log"
        logging.info(f"Logging emulator {avd_name} to {emu_log_path}...")
        emu_log = open(emu_log_path, "w")
        emu_logs.append(emu_log)
        proc = subprocess.Popen(
            [
                str(emulator_bin),
                f"@{avd_name}",
                "-no-window",
                "-no-audio",
                "-no-snapshot",
            ],
            stderr=emu_log,
            stdout=emu_log,
        )
        emu_procs.append(proc)

      # Wait for devices to appear in adb
      num_emulators = len(configs)
      logging.info("Waiting for devices to appear in adb...")
      avd_to_serial = {}
      while True:
        output = subprocess.check_output(["adb", "devices"], encoding="utf-8")
        connected_ids = [
            line.split()[0]
            for line in output.splitlines()
            if "device" in line and not line.startswith("List")
        ]

        for dev in connected_ids:
          if dev not in avd_to_serial.values():
            try:
              avd_name = subprocess.check_output(
                  [
                      "adb",
                      "-s",
                      dev,
                      "shell",
                      "getprop",
                      "ro.boot.qemu.avd_name",
                  ],
                  encoding="utf-8",
              ).strip()
              if avd_name in avds:
                avd_to_serial[avd_name] = dev
                logging.info(f"Mapped {avd_name} to {dev}")
            except subprocess.CalledProcessError:
              pass  # Device might be offline or not have property yet

        if len(avd_to_serial) >= num_emulators:
          break
        time.sleep(5)

      # Wait for boot completed and network setup on all devices
      for avd_name in avds:
        dev = avd_to_serial.get(avd_name)
        if not dev:
          logging.error(f"No ADB serial found for AVD {avd_name}!")
          return False
        logging.info(f"Waiting for {dev} boot...")
        wait_cycles = 0
        while True:
          try:
            boot_completed = subprocess.check_output(
                ["adb", "-s", dev, "shell", "getprop", "sys.boot_completed"],
                encoding="utf-8",
            ).strip()
            if boot_completed == "1":
              break
          except subprocess.CalledProcessError:
            pass
          wait_cycles += 1
          if wait_cycles > 60:  # 3 minutes timeout (60 * 3s)
            logging.error(f"{dev} failed to boot within timeout!")
            return False
          time.sleep(3)

        logging.info(f"Waiting for {dev} network stack (ping 10.0.2.2)...")
        wait_cycles = 0
        while True:
          try:
            result = subprocess.run(
                [
                    "adb",
                    "-s",
                    dev,
                    "shell",
                    "ping",
                    "-c",
                    "1",
                    "-W",
                    "1",
                    "10.0.2.2",
                ],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            if result.returncode == 0:
              break
          except Exception:
            pass
          wait_cycles += 1
          if wait_cycles > 30:  # 1 minute timeout (30 * 2s)
            logging.error(f"{dev} network stack failed to come up!")
            return False
          time.sleep(2)

        logging.info(f"Connecting {dev} to AndroidWifi...")
        subprocess.run(
            [
                "adb",
                "-s",
                dev,
                "shell",
                "cmd",
                "wifi",
                "connect-network",
                "AndroidWifi",
                "open",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )

        logging.info(f"Waiting for {dev} IP address on wlan0...")
        wait_cycles = 0
        while True:
          try:
            output = subprocess.check_output(
                ["adb", "-s", dev, "shell", "ip", "addr", "show", "wlan0"],
                encoding="utf-8",
            )
            if "inet " in output:
              break
          except subprocess.CalledProcessError:
            pass
          wait_cycles += 1
          if wait_cycles > 15:
            logging.error(f"{dev} wlan0 failed to associate or pull an IP!")
            return False
          time.sleep(2)

      # Run verify runner
      runner_bin = (
          AOSP_ROOT / "bazel-bin/external/netsim+/next/verify/runner/runner"
      )
      apk_path = (
          AOSP_ROOT
          / "bazel-bin/external/netsim+/next/verify/instrumentation/vbs/vbs.apk"
      )
      if not runner_bin.exists():
        logging.error(f"runner binary not found at {runner_bin}")
        return False
      if not apk_path.exists():
        logging.error(f"vbs.apk not found at {apk_path}")
        return False
      spec_dir = AOSP_ROOT / "tools/netsim/next/tests/features"

      logging.info("Running verify tests...")
      run(
          [
              str(runner_bin),
              "run",
              "--android-home",
              str(sdk_root),
              "--apk-path",
              str(apk_path),
              "--spec-dir",
              str(spec_dir),
          ],
          self.env,
          "verify_runner",
      )

    finally:
      logging.info("Cleaning up...")
      for proc in emu_procs:
        try:
          proc.terminate()
        except Exception as e:
          logging.warning(f"Failed to terminate emulator process: {e}")
          continue
      if netsimd_proc:
        try:
          netsimd_proc.terminate()
        except Exception as e:
          logging.warning(f"Failed to terminate netsimd process: {e}")

      # Close log files
      if netsimd_log:
        try:
          netsimd_log.close()
        except Exception as e:
          logging.warning(f"Failed to close netsimd log: {e}")
      for log in emu_logs:
        try:
          log.close()
        except Exception as e:
          logging.warning(f"Failed to close emulator log: {e}")
          continue

      for avd_name in avds:
        try:
          logging.info(f"Deleting AVD {avd_name}...")
          process = subprocess.run(
              [
                  str(avdmanager_bin),
                  "delete",
                  "avd",
                  "--name",
                  avd_name,
              ],
              check=False,
              capture_output=True,
          )
          if process.returncode != 0:
            logging.warning(
                f"Failed to delete AVD {avd_name}:"
                f" {process.stderr.decode('utf-8')}"
            )
        except Exception as e:
          logging.warning(f"Failed to delete AVD {avd_name}: {e}")

    return True
