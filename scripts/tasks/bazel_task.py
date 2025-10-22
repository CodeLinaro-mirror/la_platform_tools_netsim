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

import platform

from tasks.task import Task
from utils import AOSP_ROOT, run


class BazelTask(Task):

  def __init__(self, args, env):
    super().__init__("Bazel")
    self.env = env
    system = f"{platform.system().lower()}-x86_64"
    self.path = AOSP_ROOT / "prebuilts" / "bazel" / system / "bazel"

  def do_run(self):
    system = platform.system().lower()
    platform_configs = {
        "darwin": {
            "config": (
                "macos_x86_64" if platform.machine() == "x86_64" else "macos"
            ),
            "cwd": AOSP_ROOT / "tools" / "netsim",
            "build_targets": [":all", "//rust/...", "//next/..."],
            "test_targets": [":all", "//rust/...", "//next/..."],
        },
        "linux": {
            "config": "release",
            "cwd": AOSP_ROOT,
            "build_targets": [
                "@netsim//:all",
                "@netsim//rust/...",
                "@netsim//next/...",
            ],
            "test_targets": [
                "@netsim//:all",
                "@netsim//rust/...",
                "@netsim//next/...",
            ],
        },
    }

    cfg = platform_configs.get(system)
    if not cfg:
      print(f"Unsupported platform for bazel: {platform.system()}")
      return False

    build_config = f'--config={cfg["config"]}'

    def _run_bazel(action, targets):
      run(
          [self.path, action] + targets + [build_config],
          self.env,
          f"bazel {action}",
          cfg["cwd"],
      )

    _run_bazel("build", cfg["build_targets"])
    _run_bazel("test", cfg["test_targets"])

    return True
