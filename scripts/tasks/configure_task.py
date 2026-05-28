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
    get_bazel_path,
    get_bazel_startup_options,
    run,
)


class ConfigureTask(Task):

  def __init__(self, args, env):
    super().__init__("Configure")
    self.args = args
    self.out = Path(args.out_dir)
    self.env = env

  def do_run(self):
    return self._run_bazel()

  def _run_bazel(self):
    # For Bazel, we clean the distribution directory to ensure a fresh install.
    # This prevents stale artifacts from previous builds from interfering.
    dist_dir = self.out / "distribution"
    if dist_dir.exists():
      shutil.rmtree(dist_dir)

    if self.args.clean:
      bazel = get_bazel_path()
      startup_options = get_bazel_startup_options()
      run(
          [bazel] + startup_options + ["clean", "--expunge"],
          self.env,
          "bazel clean",
          AOSP_ROOT,
      )
    return True
