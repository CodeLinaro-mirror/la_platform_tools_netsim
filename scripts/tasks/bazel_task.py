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

import os
from pathlib import Path
import platform

from tasks.task import Task
from utils import AOSP_ROOT, run


class BazelTask(Task):

  DEFAULT_TARGETS = [
      "@netsim//:all",
      "@netsim//rust/...",
      "@netsim//next/...",
  ]

  def __init__(self, args, env):
    super().__init__("Bazel")
    self.out = Path(args.out_dir)
    self.env = env
    self.env.tmp_dir = (
        Path(os.environ.get("TMPDIR")) if os.environ.get("TMPDIR") else None
    )
    self.buildbot = args.buildbot
    self.hermetic = args.hermetic
    # TODO(b/320434273): Include next/... for windows once dependent crates are imported
    if platform.system().lower() == "windows":
      self.DEFAULT_TARGETS = [
          "@netsim//:all",
          "@netsim//rust/...",
      ]
    self.targets = args.bazel_targets or self.DEFAULT_TARGETS
    system = f"{platform.system().lower()}-x86_64"
    self.path = AOSP_ROOT / "prebuilts" / "bazel" / system / "bazel"

  def _run_gcloud_auth(self):
    # This is required for hermetic builds to access GCS for dependencies.
    # Check if we already have credentials to avoid browser popup
    if (
        run(
            [
                "gcloud",
                "auth",
                "application-default",
                "print-access-token",
            ],
            self.env,
            "gcloud auth check",
            AOSP_ROOT,
            throw_on_failure=False,
            log_output=False,
        )
        == 0
    ):
      print("Gcloud already authenticated, skipping login")
    else:
      run(
          [
              "gcloud",
              "auth",
              "application-default",
              "login",
              "--project=emulator-builds",
          ],
          self.env,
          "gcloud auth",
          AOSP_ROOT,
      )
    # This is required to access the quota project for GCS dependencies.
    run(
        [
            "gcloud",
            "auth",
            "application-default",
            "set-quota-project",
            "emulator-builds",
        ],
        self.env,
        "gcloud auth",
        AOSP_ROOT,
    )

  def do_run(self):
    configs = ["release"]
    if self.buildbot:
      configs.append("ci")
    elif self.hermetic:
      self._run_gcloud_auth()
      configs.append("hermetic")

    build_configs = [f"--config={c}" for c in configs]
    if platform.system().lower() == "windows":
      # Force Static CRT linking to avoid ABI mismatches with the Emulator's prebuilt DLLs.
      build_configs.append("--features=static_link_msvcrt")

    startup_options = []
    if self.env.tmp_dir:
      startup_options += [
          f"--output_base={self.env.tmp_dir / 'output'}",
          f"--install_base={self.env.tmp_dir / 'install'}",
      ]

    def _run_bazel(action: list[str], targets, extra_args=[]):
      run(
          [self.path]
          + startup_options
          + action
          + targets
          + build_configs
          + extra_args,
          self.env,
          f"bazel {' '.join(action)}",
          AOSP_ROOT,
      )

    _run_bazel(["build"], self.targets)
    _run_bazel(["test"], self.targets, extra_args=["--test_output=streamed"])

    return True
