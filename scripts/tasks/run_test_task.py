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

from pathlib import Path
import platform

from tasks.task import Task
from utils import (AOSP_ROOT, run, rust_version)

PLATFORM_SYSTEM = platform.system()


class RunTestTask(Task):

  def __init__(self, args, env):
    super().__init__("RunTest")
    self.buildbot = args.buildbot
    self.out = Path(args.out_dir)
    self.env = env

  def do_run(self):
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
    for package in [
        "hostapd-rs",
        "libslirp-rs",
        "http-proxy",
        "netsim-cli",
        "netsim-common",
        "netsim-daemon",
        "netsim-packets",
        "capture",
    ]:
      # TODO(b/379708365): Resolve netsim-daemon test for Mac & Windows
      if (
          package in ["netsim-daemon", "netsim-cli"]
          and PLATFORM_SYSTEM != "Linux"
      ):
        continue
      cmd = [script, package, str(self.out), rust_version()]
      run(cmd, self.env, f"{package}_unit_tests")
    return True
