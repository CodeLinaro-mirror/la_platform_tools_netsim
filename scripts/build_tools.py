#!/usr/bin/env python3
#
# Copyright 2023 - The Android Open Source Project
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
from __future__ import absolute_import, division, print_function

import argparse
import glob
import logging
import os
from pathlib import Path
import platform
import shutil
import sys
import zipfile

from install_emulator import InstallEmulatorManager
from server_config import ServerConfig
from utils import (
    AOSP_ROOT,
    EMULATOR_ARTIFACT_PATH,
    cmake_toolchain,
    config_logging,
    create_emulator_artifact_path,
    is_presubmit,
    log_system_info,
    platform_to_cmake_target,
    run,
)


def fetch_build_chaining_artifacts(out, presubmit):
  """Fetch the Emulator prebuilts for build_bots (go/build_chaining)"""
  try:
    prebuilt_path = out / "prebuilt_cached" / "artifacts"
    files = glob.glob(str(prebuilt_path / f"*.zip"))
    for file in files:
      shutil.copy2(prebuilt_path / file, EMULATOR_ARTIFACT_PATH)
  except Exception as e:
    if presubmit:
      raise e
    else:
      logging.warn(
          f"An error ocurred during fetch_build_chaining_artifacts: {e}"
      )


def main():
  config_logging()
  log_system_info()
  create_emulator_artifact_path()

  parser = argparse.ArgumentParser(
      description=(
          "Configures the android netsim cmake project so it can be build"
      )
  )
  parser.add_argument(
      "--out_dir", type=str, required=True, help="The output directory"
  )
  parser.add_argument(
      "--dist_dir", type=str, required=True, help="The destination directory"
  )
  parser.add_argument(
      "--build-id",
      type=str,
      default=[],
      required=True,
      dest="build_id",
      help="The netsim build number",
  )
  parser.add_argument(
      "--target",
      type=str,
      default=platform.system(),
      help="The build target, defaults to current os",
  )
  parser.add_argument(
      "--enable_system_rust",
      action="store_true",
      help="Build the netsim with the System Rust on the host machine",
  )
  parser.add_argument(
      "--with_debug", action="store_true", help="Build debug instead of release"
  )
  parser.add_argument(
      "--buildbot", action="store_true", help="Invoked by Android buildbots"
  )

  args = parser.parse_args()

  os.environ["GIT_DISCOVERY_ACROSS_FILESYSTEM"] = "1"

  target = platform.system().lower()

  presubmit = is_presubmit(args.build_id)

  if args.target:
    target = args.target.lower()

  if not os.path.isabs(args.out_dir):
    args.out_dir = os.path.join(AOSP_ROOT, args.out_dir)

  out = Path(args.out_dir)
  if out.exists():
    # Fetch Emulator Artifacts
    fetch_build_chaining_artifacts(out, presubmit)
    # Clear out_dir
    shutil.rmtree(out)
  out.mkdir(exist_ok=True, parents=True)

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
      f"-B{out}",
      "-G Ninja",
      f"-DCMAKE_TOOLCHAIN_FILE={cmake_toolchain(target)}",
      AOSP_ROOT / "tools" / "netsim",
  ]

  # Make sure the dist directory exists.
  dist = Path(args.dist_dir).absolute()
  dist.mkdir(exist_ok=True, parents=True)

  with ServerConfig(presubmit, args) as cfg:
    # Turn on sccache?
    # if cfg.sccache:
    #    launcher.append(f"-DOPTION_CCACHE=${cfg.sccache}")

    # Configure
    run(launcher, cfg.get_env(), "bld")

    # Build
    run(
        [cmake, "--build", out, "--target", "install"],
        cfg.get_env(),
        "bld",
    )

    # Zip results..
    zip_fname = (
        dist / f"netsim-{platform_to_cmake_target(target)}-{args.build_id}.zip"
    )
    search_dir = out / "distribution" / "emulator"
    logging.info("Creating zip file: %s", zip_fname)
    with zipfile.ZipFile(
        zip_fname, "w", zipfile.ZIP_DEFLATED, allowZip64=True
    ) as zipf:
      logging.info("Searching %s", search_dir)
      for fname in search_dir.glob("**/*"):
        arcname = fname.relative_to(search_dir)
        logging.info("Adding %s as %s", fname, arcname)
        zipf.write(fname, arcname)

    logging.info("Build completed!")

    # Install Emulator artifacts
    try:
      install_emulator_manager = InstallEmulatorManager(True, args.out_dir)
      install_emulator_manager.process()
    except Exception as e:
      if presubmit:
        raise e
      else:
        logging.warn(
            f"An error ocurred when processing InstallEmulatorManager: {e}"
        )

    # TODO(b/328281760): Run E2E integration tests in external/adt-infra
    logging.info("TODO(b/328281760): Enable E2E PyTests")


if __name__ == "__main__":
  main()
