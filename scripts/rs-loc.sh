#!/bin/bash

# Copyright 2023 The Android Open Source Project
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


# Report lines of code

# git checkout `git rev-list -n 1 --before="2023-01-01 12:00" main`

rust=`git ls-files | grep 'rs$' | xargs cat | wc -l`
cc=`git ls-files | grep '\.h$\|\.cc$' | xargs cat | wc -l`
cc_percent=$(( (${cc} * 100)/(${rust} + ${cc}) ))
rust_percent=$(( (${rust} * 100)/(${rust} + ${cc}) ))

echo "cc ${cc} ${cc_percent}%"
echo "rs ${rust} ${rust_percent}%"

