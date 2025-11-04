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

import logging
from pathlib import Path
import platform
import zipfile

from tasks.task import Task
from utils import AOSP_ROOT, is_bazel_build, platform_to_cmake_target


class ZipArtifactTask(Task):

  def __init__(self, args):
    super().__init__("ZipArtifact")
    self.build_id = args.build_id
    self.out = Path(args.out_dir)
    if args.target:
      self.target = args.target.lower()
    else:
      self.target = platform.system().lower()
    self.dist = Path(args.dist_dir).absolute()
    self.bazel = is_bazel_build(args)

  def do_run(self):
    # Make sure the dist directory exists.
    self.dist.mkdir(exist_ok=True, parents=True)

    # Zip results..
    zip_fname = (
        self.dist
        / f"netsim-{platform_to_cmake_target(self.target)}-{self.build_id}.zip"
    )
    if self.bazel:
      # Collect binary from toplevel build for linux.
      if self.target.startswith("linux"):
        search_dir = AOSP_ROOT / "bazel-bin" / "external" / "netsim+"
      else:
        search_dir = AOSP_ROOT / "tools" / "netsim" / "bazel-bin"
      search_glob = list((search_dir / "netsim-ui").glob("**/*"))
      search_glob.extend([search_dir / "netsim", search_dir / "netsimd"])
    else:
      search_dir = self.out / "distribution" / "emulator"
      search_glob = search_dir.glob("**/*")

    logging.info("Creating zip file: %s", zip_fname)
    with zipfile.ZipFile(
        zip_fname, "w", zipfile.ZIP_DEFLATED, allowZip64=True
    ) as zipf:
      logging.info("Searching %s", search_dir)
      for fname in search_glob:
        arcname = fname.relative_to(search_dir)
        logging.info("Adding %s as %s", fname, arcname)
        zipf.write(fname, arcname)
    return True
