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
from typing import Mapping

from tasks.compile_install_task import CompileInstallTask
from tasks.compile_task import CompileTask
from tasks.configure_task import ConfigureTask
from tasks.install_emulator_task import InstallEmulatorTask
from tasks.run_pytest_task import RunPyTestTask
from tasks.task import Task
from tasks.zip_artifact_task import ZipArtifactTask

TASK_LIST = [
    "Configure",
    "Compile",
    "CompileInstall",
    "InstallEmulator",
    "RunPyTest",
    "LocalRunAll",
]


def log_enabled_tasks(tasks):
  enabled_tasks = [
      task_name for task_name, task in tasks.items() if task.enabled
  ]
  logging.info(f"Enabled Tasks are {enabled_tasks}")


def get_tasks(args, env) -> Mapping[str, Task]:
  """A list of tasks that should be executed"""

  # Mapping of tasks
  tasks = {
      "Configure": ConfigureTask(args, env),
      "Compile": CompileTask(args, env),
      "CompileInstall": CompileInstallTask(args, env),
      "ZipArtifact": ZipArtifactTask(args),
      "InstallEmulator": InstallEmulatorTask(args),
      "RunPyTest": RunPyTestTask(args),
  }

  # Enable all tasks for buidlbots
  if args.buildbot:
    for task_name in [
        "Configure",
        "CompileInstall",
        "ZipArtifact",
        "InstallEmulator",
        "RunPyTest",
    ]:
      tasks[task_name].enable(True)
    return tasks

  if args.task:
    # Enable user specified tasks
    for args_task_name in args.task:
      if args_task_name.lower() == "localrunall":
        # We don't need installation process when running locally
        for task_name in [
            "Configure",
            "Compile",
            "InstallEmulator",
            "RunPyTest",
        ]:
          tasks[task_name].enable(True)
        break
      elif args_task_name.lower() == "configure":
        tasks["Configure"].enable(True)
      elif args_task_name.lower() == "compile":
        tasks["Compile"].enable(True)
      elif args_task_name.lower() == "compileinstall":
        tasks["CompileInstall"].enable(True)
      elif args_task_name.lower() == "zipartifact":
        tasks["ZipArtifact"].enable(True)
      elif args_task_name.lower() == "installemulator":
        tasks["InstallEmulator"].enable(True)
      elif args_task_name.lower() == "fullbuild":
        tasks["Configure"].enable(True)
        tasks["Compile"].enable(True)
        tasks["InstallEmulator"].enable(True)
      elif args_task_name.lower() == "runpytest":
        tasks["RunPyTest"].enable(True)
      else:
        logging.error(f"Unknown task: {args_task_name}")
  else:
    # If task argument isn't passed, only enable ConfigureTask
    tasks["Configure"].enable(True)
  return tasks
