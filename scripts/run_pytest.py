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

import argparse
import logging
import platform

from environment import get_default_environment
from utils import (
    AOSP_ROOT,
    EMULATOR_ARTIFACT_PATH,
    binary_extension,
    config_logging,
    log_system_info,
    run,
)

PYTEST_DIR = AOSP_ROOT / "external" / "adt-infra" / "pytest" / "test_embedded"
OBJS_DIR = AOSP_ROOT / "tools" / "netsim" / "objs"


class RunPytestManager:
  """Manager for running e2e integration pytests with Emulator

  The prerequisite is that the emulator installation has to be completed.
  RunPytestManager runs a run_tests shell script in external/adt-infra.
  It will take the emulator binary installed as the argument for the
  script.

  Attributes:

  buildbot: A boolean indicating if it's being invoked with Android Build
    Bots
  """

  def __init__(self, buildbot):
    """Initializes the instances based on environment

    Args:
        buildbot: Defines if it's being invoked with Build Bots and defines
          self.dir as the directory of the emulator binary
    """
    self.dir = EMULATOR_ARTIFACT_PATH / "emulator" if buildbot else OBJS_DIR

  def process(self):
    """Process the emulator e2e pytests

    The process will check if the emulator installation occurred
    and run the run_tests.sh script.
    """
    emulator_bin = self.dir / binary_extension("emulator")
    if not (self.dir.exists() and emulator_bin.exists()):
      logging.info("Please install emulator before running pytests")
      return
    if platform.system() == "Windows":
      run_tests_script = PYTEST_DIR / "run_tests.cmd"
    else:
      run_tests_script = PYTEST_DIR / "run_tests.sh"
    cmd = [
        run_tests_script,
        "--emulator",
        emulator_bin,
        "--test_config",
        PYTEST_DIR / "cfg" / "netsim_tests.json",
        "--failures_as_errors",
    ]
    run(cmd, get_default_environment(AOSP_ROOT), "e2e_pytests")


def main():
  config_logging()
  log_system_info()

  # Argument parsing
  parser = argparse.ArgumentParser(description="Emulator installation script")
  parser.add_argument(
      "--buildbot",
      action="store_true",
      help="Checking the usage of build bots",
  )
  args = parser.parse_args()

  # Running Pytests
  run_pytest_manager = RunPytestManager(args.buildbot)
  run_pytest_manager.process()


if __name__ == "__main__":
  main()
