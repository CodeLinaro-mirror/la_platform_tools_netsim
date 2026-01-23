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
import shutil

from tasks.task import Task
from utils import (
    AOSP_ROOT,
    CMAKE,
    WINDOWS_TMP_OBJS_PATH,
    get_bazel_path,
    move_contents,
    run,
    run_gcloud_auth,
)


class CompileTask(Task):

  def __init__(self, args, env):
    super().__init__("Compile")
    self.args = args
    self.out = Path(args.out_dir)
    self.env = env

  def do_run(self):
    if self.args.cmake:
      return self._run_cmake()
    return self._run_bazel()

  def _run_bazel(self):
    bazel = get_bazel_path()
    configs = ["release"]
    if self.args.buildbot:
      configs.append("ci")
    elif self.args.hermetic:
      run_gcloud_auth(self.env)
      configs.append("hermetic")

    build_configs = [f"--config={c}" for c in configs]

    startup_options = []
    tmp_dir = getattr(self.env, "tmp_dir", None)
    if tmp_dir:
      startup_options += [
          f"--output_base={tmp_dir / 'output'}",
          f"--install_base={tmp_dir / 'install'}",
      ]

    # Default targets
    targets = self.args.bazel_targets or [
        "@netsim//:all",
        "@netsim//rust/...",
        "@netsim//next/...",
    ]
    # TODO(b/320434273): Include next/... for windows once dependent crates are imported
    if platform.system().lower() == "windows":
      targets = self.args.bazel_targets or [
          "@netsim//:all",
          "@netsim//rust/...",
      ]

    run(
        [bazel] + startup_options + ["build"] + targets + build_configs,
        self.env,
        "bazel build",
        AOSP_ROOT,
    )
    return True

  def _run_cmake(self):
    if platform.system() == "Windows":
      try:
        # Use mkdir() with parents=True and exist_ok=True
        WINDOWS_TMP_OBJS_PATH.mkdir(parents=True, exist_ok=True)
        print(
            f"Directory '{WINDOWS_TMP_OBJS_PATH}' ensured (created or already"
            " exists)."
        )

      except OSError as e:
        # Catch potential OS errors (like permission issues)
        print(f"Error creating directory '{WINDOWS_TMP_OBJS_PATH}': {e}")
      run(
          [CMAKE, "--build", WINDOWS_TMP_OBJS_PATH],
          self.env,
          "bld",
      )
      move_contents(
          WINDOWS_TMP_OBJS_PATH,
          self.out,
      )
    else:
      run(
          [CMAKE, "--build", self.out],
          self.env,
          "bld",
      )
    return True
