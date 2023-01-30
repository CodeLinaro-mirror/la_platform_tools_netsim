#!/bin/bash
# Copyright 2022 The Android Open Source Project
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

# Formats source files according to Google's style guide.

# Run clang-format.
find src \( -name '*.cc' -o -name '*.h' -o -name '*.proto' \) \
  -exec clang-format -i {} \;

# Format rust.
REPO=$(dirname "$0")/../../..
$REPO/prebuilts/rust/linux-x86/stable/rustfmt -v -- $REPO/tools/netsim/rust/*/src/*.rs
