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
    get_bazel_build_configs,
    get_bazel_path,
    get_bazel_startup_options,
    get_bazel_targets,
    run,
)


class BuildTask(Task):

  def __init__(self, args, env):
    super().__init__("Build")
    self.args = args
    self.out = Path(args.out_dir)
    self.env = env

  def do_run(self):
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
