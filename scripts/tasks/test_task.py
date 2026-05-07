#!/usr/bin/env python3
# Copyright 2024 - The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

#

import logging
from pathlib import Path
import platform
import shutil
import subprocess
import xml.etree.ElementTree as ET

from tasks.task import Task
from utils import (
    AOSP_ROOT,
    get_bazel_build_configs,
    get_bazel_path,
    get_bazel_startup_options,
    get_bazel_targets,
    run,
    rust_version,
)

PLATFORM_SYSTEM = platform.system()
ALL_PACKAGES = [
    "hostapd-rs",
    "libslirp-rs",
    "http-proxy",
    "netsim-cli",
    "netsim-common",
    "netsim-daemon",
    "netsim-packets",
    "capture",
]


class TestTask(Task):

  def __init__(self, args, env):
    super().__init__("Test")
    self.args = args
    self.buildbot = args.buildbot
    self.out = Path(args.out_dir)
    self.crate = args.crate
    self.env = env

  def do_run(self):
    if not self.args.cmake:
      # Bazel Test
      bazel = get_bazel_path()
      build_configs = get_bazel_build_configs(self.args, self.env)
      startup_options = get_bazel_startup_options()
      targets = get_bazel_targets(self.args)

      try:
        run(
            [bazel]
            + startup_options
            + ["test"]
            + targets
            + build_configs
            + ["--test_output=streamed"],
            self.env,
            "bazel test",
            AOSP_ROOT,
        )
      except Exception as e:
        if self.buildbot:
          self._copy_bazel_test_logs()
        raise e
      return True

    # TODO(b/379745416): Support clippy for Mac and Windows
    if PLATFORM_SYSTEM == "Linux":

      def run_clippy(flags):
        run(
            [
                AOSP_ROOT / "tools" / "netsim" / "scripts" / "cargo_clippy.sh",
                str(self.out),
                rust_version(),
                " ".join(flags),
            ],
            self.env,
            "clippy",
        )

      # Default Rust lints in Android. Reference: build/soong/rust/config/lints.go
      default_clippy_flags = [
          # Default rustc Lints from main build
          "-A deprecated",
          "-A unknown_lints",
          "-D missing-docs",
          "-D warnings",
          "-D unsafe_op_in_unsafe_fn",
          # Default Clippy lints from main build
          "-A clippy::disallowed_names",
          "-A clippy::empty_line_after_doc_comments",
          "-A clippy::type-complexity",
          # TODO: Enable once prebuilt clippy is updated to 1.75.0+
          # "-A clippy::unnecessary_fallible_conversions",
          "-A clippy::unnecessary-wraps",
          "-A clippy::unusual-byte-groupings",
          "-A clippy::upper-case-acronyms",
          "-D clippy::undocumented_unsafe_blocks",
      ]
      # Additional lints for our project.
      additional_clippy_flags = [
          "-W clippy::cognitive-complexity",
      ]
      # Run cargo clippy with default flags
      run_clippy(default_clippy_flags)
      # Run cargo clippy with additional flags
      run_clippy(additional_clippy_flags)

    # Set script for cargo Test
    if PLATFORM_SYSTEM == "Windows":
      script = AOSP_ROOT / "tools" / "netsim" / "scripts" / "cargo_test.cmd"
    else:
      script = AOSP_ROOT / "tools" / "netsim" / "scripts" / "cargo_test.sh"

    # Run cargo Test
    packages = self.crate if self.crate else ALL_PACKAGES
    for package in packages:
      # TODO(b/379708365): Resolve netsim-daemon test for Mac & Windows
      if (
          package in ["netsim-daemon", "netsim-cli"]
          and PLATFORM_SYSTEM != "Linux"
      ):
        continue
      cmd = [script, package, str(self.out), rust_version()]
      run(cmd, self.env, f"{package}_unit_tests")
    return True

  def _copy_bazel_test_logs(self):
    bazel = get_bazel_path()
    startup_options = get_bazel_startup_options()

    # Query Bazel for the actual testlogs directory
    try:
      result = subprocess.run(
          [str(bazel)] + startup_options + ["info", "testlogs"],
          capture_output=True,
          text=True,
          cwd=AOSP_ROOT,
          env=self.env,
          check=True,
      )
      testlogs_dir = Path(result.stdout.strip())
    except subprocess.CalledProcessError as e:
      logging.warning(f"Failed to get testlogs dir via bazel info: {e}")
      testlogs_dir = AOSP_ROOT / "bazel-testlogs"

    dest_dir = Path(self.args.dist_dir).absolute() / "logs" / "bazel-logs"

    if testlogs_dir.exists():
      logging.info(
          f"Scanning Bazel test logs in {testlogs_dir} for failures..."
      )
      for xml_path in testlogs_dir.glob("**/test.xml"):
        try:
          tree = ET.parse(xml_path)
          root = tree.getroot()
          failed = False
          if root.tag == "testsuites":
            for suite in root.findall("testsuite"):
              failures = int(suite.get("failures", 0))
              errors = int(suite.get("errors", 0))
              if failures > 0 or errors > 0:
                failed = True
                break
          elif root.tag == "testsuite":
            failures = int(root.get("failures", 0))
            errors = int(root.get("errors", 0))
            if failures > 0 or errors > 0:
              failed = True

          log_path = xml_path.parent / "test.log"
          if failed and log_path.exists():
            rel_path = log_path.relative_to(testlogs_dir)
            dest_path = dest_dir / rel_path
            dest_path.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(log_path, dest_path)
            logging.info(f"Copied failed test log to {dest_path}")
        except ET.ParseError as e:
          logging.warning(f"Failed to parse XML {xml_path}: {e}")
        except ValueError as e:
          logging.warning(f"Value error processing {xml_path}: {e}")
        except OSError as e:
          logging.warning(f"I/O error processing {xml_path}: {e}")
    else:
      logging.warning(f"Bazel test logs directory not found at {testlogs_dir}")
