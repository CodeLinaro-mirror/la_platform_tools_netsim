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
    if platform.system().lower() == "darwin":
      if platform.machine() == "x86_64":
        config = "macos_x86_64"
      else:
        config = "macos"
    else:
      config = "linux"

    # Build
    run(
        [
            self.path,
            "build",
            ":all",
            "//rust/...",
            "//next/...",
            "--config=" + config,
        ],
        self.env,
        "bazel build",
        AOSP_ROOT / "tools" / "netsim",
    )

    # Test
    run(
        [
            self.path,
            "test",
            ":all",
            "//rust/...",
            "//next/...",
            "--config=" + config,
        ],
        self.env,
        "bazel test",
        AOSP_ROOT / "tools" / "netsim",
    )

    return True
