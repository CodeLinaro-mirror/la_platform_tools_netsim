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
    self.buildbot = args.buildbot
    system = f"{platform.system().lower()}-x86_64"
    self.path = AOSP_ROOT / "prebuilts" / "bazel" / system / "bazel"

  def do_run(self):
    configs = ["release"]
    if self.buildbot:
      configs.append("ci")

    build_configs = [f"--config={c}" for c in configs]

    targets = [
        "@netsim//:all",
        "@netsim//rust/...",
        "@netsim//next/...",
    ]

    def _run_bazel(action, targets):
      run(
          [self.path, action] + targets + build_configs,
          self.env,
          f"bazel {action}",
          AOSP_ROOT,
      )

    _run_bazel("build", targets)
    _run_bazel("test", targets)

    return True