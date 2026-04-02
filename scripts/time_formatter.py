#!/usr/bin/env python
# Copyright 2021 - The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

#
import datetime
import logging
import time


class TimeFormatter(logging.Formatter):
  """A formatter used by the build system that:

  - Strips whitespace.
  - Formats time since start
  """

  def __init__(self, fmt=None):
    self.start_time = time.time()
    super(TimeFormatter, self).__init__(fmt)

  def formatTime(self, record, datefmt=None):
    fmt = datefmt or "%H:%M:%S"
    ct = self.converter(record.created)
    dt = datetime.timedelta(seconds=record.created - self.start_time)
    mm, ss = divmod(dt.total_seconds(), 60)
    _, mm = divmod(mm, 60)
    # 2 digit precision is sufficient.
    dt_fmt = "%02d:%02d.%-2d" % (mm, ss, dt.microseconds % 100)
    return "{}({})".format(time.strftime(fmt, ct), dt_fmt)

  def format(self, record):
    record.msg = str(record.msg).strip()
    return super(TimeFormatter, self).format(record)
