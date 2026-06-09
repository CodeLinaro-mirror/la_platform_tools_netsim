#!/usr/bin/env python3
# Copyright 2024 - The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

#

import logging
from pathlib import Path
import platform
import zipfile

from tasks.task import Task
from utils import AOSP_ROOT, platform_to_target_name


class ZipArtifactTask(Task):

  def __init__(self, args):
    super().__init__("ZipArtifact")
    self.build_id = args.build_id
    self.out = Path(args.out_dir)
    self.target = args.target or platform.system()
    self.dist = Path(args.dist_dir).absolute()

  def do_run(self):
    # Make sure the dist directory exists.
    self.dist.mkdir(exist_ok=True, parents=True)

    # Zip results..
    zip_fname = (
        self.dist
        / f"netsim-{platform_to_target_name(self.target)}-{self.build_id}.zip"
    )
    search_dir = self.out / "distribution" / "emulator"
    if not search_dir.is_dir():
      logging.warning(
          f"Artifact directory not found: {search_dir}. Skipping zip."
      )
      return True
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
