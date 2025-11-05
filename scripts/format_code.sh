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
# By default, formats all files.
# Use --diff to format files that are different from HEAD.

set -euo pipefail

# Go to the root of the git repository (tools/netsim).
cd "$(dirname "$0")/.."

# Argument parsing
FORMAT_ALL=true
if [[ "$*" == *"--diff"* ]]; then
  FORMAT_ALL=false
fi

REPO="$(dirname "$0")/../../.."
OS=$(uname | tr '[:upper:]' '[:lower:]')

# Function to run a formatter in the background
format() {
  local name="$1"
  local cmd="$2"
  shift 2
  local files=("$@")

  if [ ${#files[@]} -gt 0 ]; then
    echo "Formatting ${#files[@]} $name files..."
    $cmd "${files[@]}" &
    pids+=($!)
  fi
}

# Populate file lists based on mode
if $FORMAT_ALL; then
  echo "Gathering all files to format..."
  mapfile -d '' clang_files < <(find src proto ui/ts -type f \( -name '*.cc' -o -name '*.h' -o -name '*.proto' -o -name '*.ts' \) -print0)
  mapfile -d '' rust_files < <(find rust next -type f -name '*.rs' -not -path "*/target/*" -print0)
  mapfile -d '' java_files < <(find . -type f -name '*.java' -not -path '*/target/*' -not -path './.git/*' -not -path './bazel-out/*' -not -path './objs/*' -print0)
  mapfile -d '' py_files < <(find . -type f -name '*.py' -not -path '*/target/*' -not -path './.git/*' -not -path './bazel-out/*' -not -path './objs/*' -print0)
  mapfile -d '' cmake_files < <(find . -type f \( -name 'CMakeLists.txt' -o -name '*.cmake' \) -not -path '*/target/*' -not -path './.git/*' -not -path './bazel-out/*' -not -path './objs/*' -print0)
  mapfile -d '' bp_files < <(find . -maxdepth 1 -type f -name "Android.bp" -print0)
  mapfile -d '' bazel_files < <(find . -type f \( -name "BUILD" -o -name "MODULE.bazel" -o -name "BUILD.bazel" \) -not -path '*/target/*' -not -path './.git/*' -not -path './bazel-out/*' -not -path './objs/*' -print0)
else
  echo "Gathering changed files to format..."
  mapfile -t files < <(git diff --name-only --diff-filter=ACMRTUXB HEAD && git ls-files --others --exclude-standard)

  if [ ${#files[@]} -eq 0 ]; then
    echo "No changed files to format."
    exit 0
  fi

  for f in "${files[@]}"; do
    [[ "$f" =~ \.(cc|h|proto|ts)$ ]] && clang_files+=("$f")
    [[ "$f" =~ \.rs$ ]] && rust_files+=("$f")
    [[ "$f" =~ \.java$ ]] && java_files+=("$f")
    [[ "$f" =~ \.py$ ]] && py_files+=("$f")
    [[ "$f" =~ CMakeLists\.txt$|\.cmake$ ]] && cmake_files+=("$f")
    [[ "$f" =~ Android\.bp$ ]] && bp_files+=("$f")
    [[ "$f" =~ BUILD$|MODULE\.bazel$|BUILD\.bazel$ ]] && bazel_files+=("$f")
  done
fi

# Run formatters in parallel
pids=()
RUSTFMT="$REPO/prebuilts/rust/$OS-x86/stable/rustfmt"
BPFMT="$REPO/prebuilts/build-tools/$OS-x86/bin/bpfmt"

format "C/C++/Proto/TS" "clang-format -i" "${clang_files[@]}"
format "Rust" "$RUSTFMT --files-with-diff" "${rust_files[@]}"
format "Java" "google-java-format -i" "${java_files[@]}"
format "Python" "pyformat --in_place --alsologtostderr --noshowprefixforinfo" "${py_files[@]}"
format "CMake" "cmake-format -i" "${cmake_files[@]}"

if [ -f "$BPFMT" ]; then
  format "Android.bp" "$BPFMT -w" "${bp_files[@]}"
fi

if command -v buildifier &> /dev/null; then
  format "Bazel" "buildifier -lint=fix" "${bazel_files[@]}"
else
  echo "buildifier not found, skipping Bazel file formatting."
fi

echo "Waiting for formatters to finish..."
for pid in "${pids[@]}"; do
  wait "$pid"
done

echo "Formatting complete."