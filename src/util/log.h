/*
 * Copyright 2022 The Android Open Source Project
 * SPDX-License-Identifier: Apache-2.0
 */

#pragma once
#include <cstdarg>
#include <functional>
#include <string>
namespace netsim {

#define BtsLog(fmt, ...) __BtsLog(3, __FILE__, __LINE__, fmt, ##__VA_ARGS__)
#define BtsLogInfo(fmt, ...) __BtsLog(2, __FILE__, __LINE__, fmt, ##__VA_ARGS__)
#define BtsLogWarn(fmt, ...) __BtsLog(1, __FILE__, __LINE__, fmt, ##__VA_ARGS__)
#define BtsLogError(fmt, ...) \
  __BtsLog(0, __FILE__, __LINE__, fmt, ##__VA_ARGS__)

void __BtsLog(int priority, const char *file, int line, const char *fmt, ...);

using BtsLogFn = std::function<void(int, const char *, int, const char *)>;

void setBtsLogSink(BtsLogFn logFn);

}  // namespace netsim
