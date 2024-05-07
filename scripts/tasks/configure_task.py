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
from utils import (AOSP_ROOT, cmake_toolchain, run)


class ConfigureTask(Task):
  BUILDCONFIG = {
      "debug": "-DCMAKE_BUILD_TYPE=Debug",
      "release": "-DCMAKE_BUILD_TYPE=Release",
  }

  def __init__(self, args, env):
    super().__init__("Configure")
    self.out = Path(args.out_dir)
    self.env = env
    if args.target:
      self.target = args.target.lower()
    else:
      self.target = platform.system().lower()
    self.build_config = self.BUILDCONFIG[args.config]

  def do_run(self):
    if self.out.exists():
      shutil.rmtree(self.out)
    self.out.mkdir(exist_ok=True, parents=True)
    cmake = shutil.which(
        "cmake",
        path=str(
            AOSP_ROOT
            / "prebuilts"
            / "cmake"
            / f"{platform.system().lower()}-x86"
            / "bin"
        ),
    )
    launcher = [
        cmake,
        f"-B{self.out}",
        "-G Ninja",
        self.build_config,
        f"-DCMAKE_TOOLCHAIN_FILE={cmake_toolchain(self.target)}",
        AOSP_ROOT / "tools" / "netsim",
    ]

    run(launcher, self.env, "bld")
    return True
