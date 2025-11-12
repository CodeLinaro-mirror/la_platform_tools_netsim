#!/usr/bin/env python3
#
# Copyright 2024 - The Android Open Source Project
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

import logging
from pathlib import Path
import platform
import shutil

from tasks.task import Task
from utils import AOSP_ROOT, is_bazel_build


class BazelInstallTask(Task):

  def __init__(self, args, env):
    super().__init__("BazelInstall")
    self.enabled = is_bazel_build(args)
    self.out = Path(args.out_dir)

  def do_run(self):
    search_dir = AOSP_ROOT / "bazel-bin" / "external" / "netsim+"
    dest_dir = self.out / "distribution" / "emulator"
    logging.info(f"Installing artifacts from {search_dir} to {dest_dir}")

    try:
      if not search_dir.is_dir():
        logging.error(f"Bazel output directory not found: {search_dir}")
        return False

      dest_dir.mkdir(exist_ok=True, parents=True)

      # Copy netsim binaries
      for binary in ["netsim", "netsimd"]:
        binary_name = (
            f"{binary}.exe" if platform.system() == "Windows" else binary
        )
        src_file = search_dir / binary_name
        logging.info(f"Copying {src_file} to {dest_dir}")
        shutil.copy(src_file, dest_dir / binary_name)

      # Copy netsim-ui
      ui_src_dir = search_dir / "netsim-ui"
      ui_dest_dir = dest_dir / "netsim-ui"
      logging.info(f"Copying directory {ui_src_dir} to {ui_dest_dir}")
      shutil.copytree(ui_src_dir, ui_dest_dir, dirs_exist_ok=True)

    except FileNotFoundError as e:
      logging.error(
          f"Artifact not found: {e}. A successful Bazel build is required."
      )
      raise e
    except shutil.Error as e:
      logging.error(f"Error copying artifacts: {e}")
      raise e
    return True
