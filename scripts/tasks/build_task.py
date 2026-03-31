#!/usr/bin/env python3
# Copyright 2024 - The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

#

from pathlib import Path
import platform
import shutil

from tasks.task import Task
from utils import (
    AOSP_ROOT,
    CMAKE,
    WINDOWS_TMP_OBJS_PATH,
    get_bazel_build_configs,
    get_bazel_path,
    get_bazel_startup_options,
    get_bazel_targets,
    move_contents,
    run,
)


class BuildTask(Task):

  def __init__(self, args, env):
    super().__init__("Build")
    self.args = args
    self.out = Path(args.out_dir)
    self.env = env

  def do_run(self):
    if self.args.cmake:
      return self._run_cmake()
    return self._run_bazel()

  def _run_bazel(self):
    bazel = get_bazel_path()
    build_configs = get_bazel_build_configs(self.args, self.env)
    startup_options = get_bazel_startup_options()
    targets = get_bazel_targets(self.args)

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
