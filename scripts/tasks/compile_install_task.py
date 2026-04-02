#!/usr/bin/env python3
# Copyright 2024 - The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

#

import logging
import os
from pathlib import Path
import platform
import shutil
import stat

from tasks.task import Task
from utils import (
    AOSP_ROOT,
    CMAKE,
    WINDOWS_TMP_OBJS_PATH,
    get_bazel_build_configs,
    get_bazel_path,
    get_bazel_startup_options,
    get_bazel_targets,
    move_contents,
    run,
)


class CompileInstallTask(Task):

  def __init__(self, args, env):
    super().__init__("CompileInstall")
    self.args = args
    self.out = Path(args.out_dir)
    self.env = env

  def on_rm_error(self, func, path, exc_info):
    """Error handler for ``shutil.rmtree``.

    If the error is due to an access error (read only file)
    it attempts to add write permission and then retries.

    If the error is for another reason it re-raises the error.

    Usage : ``shutil.rmtree(path, onerror=on_rm_error)``
    """
    # Is the error an access error?
    if not os.access(path, os.W_OK):
      os.chmod(path, stat.S_IWRITE)
      func(path)
    else:
      raise

  def do_run(self):
    if self.args.cmake:
      return self._run_cmake()
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

    # Bazel Install
    search_dir = AOSP_ROOT / "bazel-bin" / "external" / "netsim+"
    dest_dir = self.out / "distribution" / "emulator"
    logging.info(f"Installing artifacts from {search_dir} to {dest_dir}")

    try:
      if not search_dir.is_dir():
        logging.error(f"Bazel output directory not found: {search_dir}")
        return False

      dest_dir.mkdir(exist_ok=True, parents=True)

      # Copy netsim binaries
      binaries = {
          "netsim": "netsim",
          "netsimd": "netsimd",
          "netsimx": "next/cli/netsim",
          "netsimdx": "next/daemon/daemon",
      }

      for binary, src in binaries.items():
        if platform.system() == "Windows":
          binary_name = f"{binary}.exe"
          src_name = f"{src}.exe"
        else:
          binary_name = binary
          src_name = src

        src_file = search_dir / src_name
        logging.info(f"Copying {src_file} to {dest_dir}")
        dest_file = dest_dir / binary_name
        # Remove the file if it exists to avoid permission errors on overwrite.
        if dest_file.is_file():
          try:
            dest_file.unlink()
          except PermissionError:
            self.on_rm_error(os.unlink, str(dest_file), None)
        shutil.copy(src_file, dest_file)

      # Copy netsim-ui
      ui_src_dir = search_dir / "netsim-ui"
      ui_dest_dir = dest_dir / "netsim-ui"
      if ui_dest_dir.exists():
        shutil.rmtree(ui_dest_dir, onerror=self.on_rm_error)
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

  def _run_cmake(self):
    # Strip for non-Windows builds
    target = "install"
    if platform.system() != "Windows":
      target += "/strip"

    # Build
    if platform.system() == "Windows":
      try:
        # Use mkdir() with parents=True and exist_ok=True
        WINDOWS_TMP_OBJS_PATH.mkdir(parents=True, exist_ok=True)
        print(
            f"Directory '{WINDOWS_TMP_OBJS_PATH}' ensured (created or already"
            " exists)."
        )

      except OSError as e:
        # Catch potential OS errors (like permission issues)
        print(f"Error creating directory '{WINDOWS_TMP_OBJS_PATH}': {e}")
      run(
          [CMAKE, "--build", WINDOWS_TMP_OBJS_PATH, "--target", target],
          self.env,
          "bld",
      )
      move_contents(
          WINDOWS_TMP_OBJS_PATH,
          self.out,
      )
    else:
      run(
          [CMAKE, "--build", self.out, "--target", target],
          self.env,
          "bld",
      )
    return True
