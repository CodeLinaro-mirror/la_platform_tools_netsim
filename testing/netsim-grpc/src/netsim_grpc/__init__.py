# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

import os
import sys

# Workaround for protobuf paths not working well on Py3.
if sys.version_info[0] == 3:
  sys.path.insert(
      0, os.path.abspath(os.path.join(os.path.dirname(__file__), "proto"))
  )
  sys.path.insert(0, os.path.abspath(os.path.dirname(__file__)))
