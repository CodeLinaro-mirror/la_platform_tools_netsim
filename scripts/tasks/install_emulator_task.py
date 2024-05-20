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

import glob
import logging
import os
from pathlib import Path
import platform
import re
import shutil
import zipfile

from environment import get_default_environment
from tasks.task import Task
from utils import (
    AOSP_ROOT,
    EMULATOR_ARTIFACT_PATH,
    binary_extension,
    run,
)

OBJS_DIR = AOSP_ROOT / "tools" / "netsim" / "objs"
PLATFORM_SYSTEM = platform.system()
PLATFORM_MACHINE = platform.machine()


class InstallEmulatorTask(Task):

  def __init__(self, args):
    super().__init__("InstallEmulator")
    self.buildbot = args.buildbot
    self.out_dir = args.out_dir
    # Local fetching use only - default to emulator-linux_x64
    self.target = args.emulator_target
    # Local Emulator directory
    self.local_emulator_dir = args.local_emulator_dir

  def do_run(self):
    install_emulator_manager = InstallEmulatorManager(
        self.buildbot, self.out_dir, self.target, self.local_emulator_dir
    )
    return install_emulator_manager.process()


class InstallEmulatorManager:
  """Manager for installing emulator artifacts into netsim build

  The InstallEmulatorManager checks if the conditions are met
  to fetch the emulator artifact zip file and installs it with the
  newly built netsim. The manager contains processing logic for
  both local and pre/post submit.

  Attributes:
    buildbot: A boolean indicating if it's being invoked with Android Build Bots
    out_dir: A str or None representing the directory of out/. This is priamrily
      used for Android Build Bots.
  """

  def __init__(self, buildbot, out_dir, target, local_emulator_dir):
    """Initializes the instances based on environment

    Args:
      buildbot: Defines if it's being invoked with Build Bots
      out_dir: Defines the out directory of the build environment
      target: The emulator build target to install
    """
    self.buildbot = buildbot
    self.out_dir = out_dir
    self.target = target
    self.local_emulator_dir = local_emulator_dir

  def __os_name_fetch(self):
    """Obtains the os substring of the emulator artifact"""
    if PLATFORM_SYSTEM == "Linux" and PLATFORM_MACHINE == "x86_64":
      return "linux"
    elif PLATFORM_SYSTEM == "Darwin":
      if PLATFORM_MACHINE == "x86_64":
        return "darwin"
      elif PLATFORM_MACHINE == "arm64":
        return "darwin_aarch64"
    elif PLATFORM_SYSTEM == "Windows":
      return "windows"
    else:
      logging.info("Unsupported OS:", PLATFORM_SYSTEM, ",", PLATFORM_MACHINE)
      return None

  def __prerequisites(self) -> bool:
    """Prerequisite checks for invalid cases"""
    if self.buildbot:
      # out_dir is not provided
      if not self.out_dir:
        logging.info("Error: please specify '--out_dir' when using buildbots")
        return False
      # If out_dir does not exist
      elif not Path(self.out_dir).exists():
        logging.info(f"Error: {self.out_dir} does not exist")
        return False
    else:
      # Without buildbots, this scripts is only runnable on Linux
      # TODO: support local builds for Mac and Windows
      if PLATFORM_SYSTEM != "Linux":
        logging.info("The local case only works for Linux")
        return False
      # Check if the netsim has been built prior to install_emulator
      if not (
          OBJS_DIR.exists()
          and (OBJS_DIR / binary_extension("netsim")).exists()
          and (OBJS_DIR / binary_extension("netsimd")).exists()
      ):
        logging.info(
            "Please run 'scripts/build_tools.sh --Compile' "
            "before running InstallEmulator"
        )
        return False
    return True

  def __unzip_emulator_artifacts(self, os_name_artifact) -> bool:
    """unzips the emulator artifacts inside EMULATOR_ARTIFACT_PATH"""
    # Unzipping emulator artifacts
    zip_file_exists = False
    for filename in os.listdir(EMULATOR_ARTIFACT_PATH):
      # Check if the filename matches the pattern
      if re.match(
          rf"^sdk-repo-{os_name_artifact}-emulator-\d+\.zip$", filename
      ):
        zip_file_exists = True
        logging.info(f"Unzipping {filename}...")
        with zipfile.ZipFile(EMULATOR_ARTIFACT_PATH / filename, "r") as zip_ref:
          zip_ref.extractall(EMULATOR_ARTIFACT_PATH)
          # Preserve permission bits
          for info in zip_ref.infolist():
            filename = EMULATOR_ARTIFACT_PATH / info.filename
            original_permissions = info.external_attr >> 16
            if original_permissions:
              os.chmod(filename, original_permissions)
    # Log and return False if the artifact does not exist
    if not zip_file_exists:
      logging.info("Emulator artifact prebuilt is not found!")
      return False
    # Remove all zip files
    files = glob.glob(
        str(
            EMULATOR_ARTIFACT_PATH
            / f"sdk-repo-{os_name_artifact}-emulator-*.zip"
        )
    )
    for file in files:
      os.remove(EMULATOR_ARTIFACT_PATH / file)
    return True

  def __copy_artifacts(self, emulator_filepath):
    """Copy artifacts into desired location

    In the local case, the emulator artifacts get copied into objs/
    In the buildbot case, the netsim artifacts get copied into
      EMULATOR_ARTIFACT_PATH

    Note that the downloaded netsim artifacts are removed before copying.
    """
    # Remove all downloaded netsim artifacts
    files = glob.glob(str(emulator_filepath / "netsim*"))
    for fname in files:
      file = emulator_filepath / fname
      if os.path.isdir(file):
        shutil.rmtree(file)
      else:
        os.remove(file)
    # Copy artifacts
    if self.buildbot:
      shutil.copytree(
          Path(self.out_dir) / "distribution" / "emulator",
          emulator_filepath,
          symlinks=True,
          dirs_exist_ok=True,
      )
    else:
      shutil.copytree(
          emulator_filepath,
          OBJS_DIR,
          symlinks=True,
          dirs_exist_ok=True,
      )
      shutil.copytree(
          emulator_filepath,
          OBJS_DIR / "distribution" / "emulator",
          symlinks=True,
          dirs_exist_ok=True,
      )

  def process(self) -> bool:
    """Process the emulator installation

    The process will terminate if sub-function calls returns
    a None or False
    """
    # Obtain OS name of the artifact
    os_name_artifact = self.__os_name_fetch()
    if not os_name_artifact:
      return False

    # Invalid Case checks
    if not self.__prerequisites():
      return False

    if self.local_emulator_dir:
      # If local_emulator_dir is provided, copy the artifacts from this directory.
      self.__copy_artifacts(Path(self.local_emulator_dir))
    else:
      # Artifact fetching for local case
      if not self.buildbot:
        # Simulating the shell command
        run(
            [
                "/google/data/ro/projects/android/fetch_artifact",
                "--latest",
                "--target",
                self.target,
                "--branch",
                "aosp-emu-master-dev",
                "sdk-repo-linux-emulator-*.zip",
            ],
            get_default_environment(AOSP_ROOT),
            "install_emulator",
            cwd=EMULATOR_ARTIFACT_PATH,
        )

      # Unzipping emulator artifacts and remove zip files
      if not self.__unzip_emulator_artifacts(os_name_artifact):
        return False

      # Copy artifacts after removing downloaded netsim artifacts
      self.__copy_artifacts(EMULATOR_ARTIFACT_PATH / "emulator")

    # Remove the EMULATOR_ARTIFACT_PATH in local case
    if not self.buildbot:
      shutil.rmtree(EMULATOR_ARTIFACT_PATH, ignore_errors=True)

    logging.info("Emulator installation completed!")
    return True
