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
import platform
from typing import Mapping

from tasks.compile_install_task import CompileInstallTask
from tasks.compile_task import CompileTask
from tasks.configure_task import ConfigureTask
from tasks.install_emulator_task import InstallEmulatorTask
from tasks.run_pytest_task import RunPyTestTask
from tasks.run_test_task import RunTestTask
from tasks.task import Task
from tasks.zip_artifact_task import ZipArtifactTask

TASK_LIST = [
    "Configure",
    "Compile",
    "CompileInstall",
    "RunTest",
    "ZipArtifact",
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
      "RunTest": RunTestTask(args, env),
      "ZipArtifact": ZipArtifactTask(args),
      "InstallEmulator": InstallEmulatorTask(args),
      "RunPyTest": RunPyTestTask(args),
  }

  # Enable all tasks for buidlbots
  if args.buildbot:
    for task_name in [
        "Configure",
        "CompileInstall",
        "RunTest",
        "ZipArtifact",
        "InstallEmulator",
        "RunPyTest",
    ]:
      tasks[task_name].enable(True)
    return tasks

  # Define the complete task map declaratively.
  task_map = {
      "configure": ["Configure"],
      "compile": ["Compile"],
      "compileinstall": ["CompileInstall"],
      "runtest": ["RunTest"],
      "zipartifact": ["ZipArtifact"],
      "installemulator": ["InstallEmulator"],
      "runpytest": ["RunPyTest"],
      "fullbuild": ["Configure", "Compile", "InstallEmulator"],
      "localrunall": [
          "Configure",
          "CompileInstall",
          "RunTest",
          "InstallEmulator",
          "RunPyTest",
      ],
  }

  # Handle the default case and convert to a set for efficient lookup.
  # If `localrunall` is present, it becomes the only task.
  user_tasks = {t.lower() for t in args.task or ["configure"]}
  if "localrunall" in user_tasks:
    user_tasks = {"localrunall"}

  # A single, clean loop to enable all required tasks.
  for task_name in user_tasks:
    if task_name in task_map:
      for task_to_enable in task_map[task_name]:
        tasks[task_to_enable].enable(True)
    else:
      logging.error(f"Unknown task: {task_name}")

  return tasks
