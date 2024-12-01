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


class RunTestTask(Task):

  def __init__(self, args, env):
    super().__init__("RunTest")
    self.buildbot = args.buildbot
    self.out = Path(args.out_dir)
    self.env = env

  def do_run(self):
    # TODO(b/379745416): Support clippy for Mac and Windows
    if platform.system() == "Linux":
      # Set Clippy flags
      clippy_flags = [
          "-A clippy::disallowed_names",
          "-A clippy::type-complexity",
          "-A clippy::unnecessary-wraps",
          "-A clippy::unusual-byte-groupings",
          "-A clippy::upper-case-acronyms",
          "-W clippy::undocumented_unsafe_blocks",
          "-W clippy::cognitive-complexity",
      ]
      # Run cargo clippy
      run(
          [
              AOSP_ROOT / "tools" / "netsim" / "scripts" / "cargo_clippy.sh",
              str(self.out),
              rust_version(),
              " ".join(clippy_flags),
          ],
          self.env,
          "clippy",
      )

    # Set script for cargo Test
    if platform.system() == "Windows":
      script = AOSP_ROOT / "tools" / "netsim" / "scripts" / "cargo_test.cmd"
    else:
      script = AOSP_ROOT / "tools" / "netsim" / "scripts" / "cargo_test.sh"

    # Run cargo Test
    for package in [
        "hostapd-rs",
        "libslirp-rs",
        "http-proxy",
        "netsim-common",
        "netsim-packets",
    ]:
      cmd = [script, package, str(self.out), rust_version()]
      run(cmd, self.env, f"{package}_unit_tests")
    return True
