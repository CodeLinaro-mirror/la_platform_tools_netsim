#!/usr/bin/env python3
# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

import os
import subprocess
import sys

sys.stdout.reconfigure(newline="\n")

if build_id := os.getenv("BUILD_NUMBER"):
  suffix = f"+{build_id}"
else:
  try:
    sha = subprocess.check_output(
        ["git", "describe", "--always", "--dirty=.dirty", "--exclude=*", "--abbrev=8"],
        stderr=subprocess.DEVNULL,
        text=True,
    ).strip()
    suffix = f"-dev+{sha}"
  except (OSError, subprocess.SubprocessError):
    suffix = "-dev"

print(f"STABLE_NETSIM_VERSION_SUFFIX {suffix}")
