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

if [ -z "${BASH_VERSION}" ] || [ "${BASH_VERSION%%.*}" -lt 4 ]; then
  echo "Bash version 4+ is required. Trying to find a newer version."
  _BASH=$(which bash)
  if [ -z "${_BASH}" ] || [ "${_BASH}" = "/bin/bash" ]; then
      # If which bash is not enough, try to find it in homebrew standard locations
      if [ -x "/opt/homebrew/bin/bash" ]; then
        _BASH="/opt/homebrew/bin/bash"
      elif [ -x "/usr/local/bin/bash" ]; then
        _BASH="/usr/local/bin/bash"
      else
        echo "Could not find a newer version of bash. Please install bash 4+."
        exit 1
      fi
  fi
  exec "${_BASH}" "$0" "$@"
fi

set -euo pipefail

# Go to the root of the git repository (tools/netsim).
cd "$(dirname "$0")/.."

# Argument parsing
FORMAT_ALL=true
if [[ $# -gt 0 && "$1" == *"--diff"* ]]; then
  shift
  FORMAT_ALL=false
fi

REPO="$(dirname "$0")/../../.."
OS=$(uname | tr '[:upper:]' '[:lower:]')

DESIRED_TAPLO_VERSION="0.10.0"

# Function to run a formatter in the background
format() {
  local name="$1"
  local cmd="$2"
  shift 2
  local files=("$@")

  local executable
  executable=$(echo "$cmd" | awk '{print $1}')
  if ! command -v "$executable" &> /dev/null; then
    echo "Error: '$executable' not found, skipping $name file formatting."
    return 0
  fi

  if [ ${#files[@]} -gt 0 ]; then
    echo "Formatting ${#files[@]} $name files..."
    $cmd "${files[@]}" &
    pids+=($!)
  fi
}

# Function to check taplo-cli installation and version
check_taplo_version() {

  if ! command -v taplo &> /dev/null; then
    echo "Error: 'taplo' not found. Install with: cargo install taplo-cli --version ${DESIRED_TAPLO_VERSION} --locked"
    exit 1
  fi

  local INSTALLED_VERSION=$(taplo --version | awk '{print $2}')
  if [ "$INSTALLED_VERSION" != "$DESIRED_TAPLO_VERSION" ]; then
    echo "Error: Found taplo version ${INSTALLED_VERSION}, but ${DESIRED_TAPLO_VERSION} is required for consistent formatting."
    echo "Please install the correct version by running: cargo install taplo-cli --version ${DESIRED_TAPLO_VERSION} --locked --force"
    exit 1
  fi
}

check_taplo_version

# Populate file lists based on mode
if $FORMAT_ALL; then
  echo "Gathering all files to format..."
  mapfile -d '' clang_files < <(find src rust next proto ui/ts -type f \( -name '*.cc' -o -name '*.h' -o -name '*.proto' -o -name '*.ts' \) -print0)
  mapfile -d '' rust_files < <(find rust next -type f -name '*.rs' -not -path "*/target/*" -print0)
  mapfile -d '' java_files < <(find . -type f -name '*.java' -not -path '*/target/*' -not -path './.git/*' -not -path './bazel-out/*' -not -path './objs/*' -print0)
  mapfile -d '' py_files < <(find . -type f -name '*.py' -not -path '*/target/*' -not -path './.git/*' -not -path './bazel-out/*' -not -path './objs/*' -print0)
  mapfile -d '' cmake_files < <(find . -type f \( -name 'CMakeLists.txt' -o -name '*.cmake' \) -not -path '*/target/*' -not -path './.git/*' -not -path './bazel-out/*' -not -path './objs/*' -print0)
  mapfile -d '' bp_files < <(find . -maxdepth 1 -type f -name "Android.bp" -print0)
  mapfile -d '' bazel_files < <(find . -type f \( -name "BUILD" -o -name "MODULE.bazel" -o -name "BUILD.bazel" -o -name "*.bzl" \) -not -path '*/target/*' -not -path './.git/*' -not -path './bazel-out/*' -not -path './objs/*' -print0)
  mapfile -d '' toml_files < <(find rust next proto -type f -name 'Cargo.toml' -not -path "*/target/*" -not -path "*/bazel-bin/*" -not -path "*/bazel-netsim/*" -not -path "*/bazel-out/*" -not -path "*/objs/*" -print0)
else
  echo "Gathering changed files to format..."

  all_files=("$@")
  if [ ${#all_files[@]} -eq 0 ]; then
    echo "No changed files to format."
    exit 0
  fi

  clang_files=()
  rust_files=()
  java_files=()
  py_files=()
  cmake_files=()
  bp_files=()
  bazel_files=()
  toml_files=()
  for f in "${all_files[@]}"; do
    [[ "$f" =~ \.(cc|h|proto|ts)$ ]] && clang_files+=("$f")
    [[ "$f" =~ \.rs$ ]] && rust_files+=("$f")
    [[ "$f" =~ \.java$ ]] && java_files+=("$f")
    [[ "$f" =~ \.py$ ]] && py_files+=("$f")
    [[ "$f" =~ CMakeLists\.txt$|\.cmake$ ]] && cmake_files+=("$f")
    [[ "$f" =~ Android\.bp$ ]] && bp_files+=("$f")
    [[ "$f" =~ BUILD$|MODULE\.bazel$|BUILD\.bazel$|\.bzl$ ]] && bazel_files+=("$f")
    [[ "$f" =~ Cargo\.toml$ ]] && toml_files+=("$f")
  done
fi

check_taplo_version

# Run formatters in parallel
pids=()
RUSTFMT="$REPO/prebuilts/rust/$OS-x86/stable/rustfmt"
BPFMT="$REPO/prebuilts/build-tools/$OS-x86/bin/bpfmt"
TAPLO_CONFIG="$REPO/tools/netsim/next/taplo.toml"

[[ ${#clang_files[@]} -gt 0 ]] && format "C/C++/Proto/TS" "clang-format -i" "${clang_files[@]}"
[[ ${#rust_files[@]} -gt 0 ]] && format "Rust" "$RUSTFMT --files-with-diff" "${rust_files[@]}"
[[ ${#java_files[@]} -gt 0 ]] && format "Java" "google-java-format -i" "${java_files[@]}"
[[ ${#py_files[@]} -gt 0 ]] && format "Python" "pyformat --in_place --alsologtostderr --noshowprefixforinfo" "${py_files[@]}"
[[ ${#cmake_files[@]} -gt 0 ]] && format "CMake" "cmake-format -i" "${cmake_files[@]}"
[[ ${#bp_files[@]} -gt 0 ]] && format "Android.bp" "$BPFMT -w" "${bp_files[@]}"
[[ ${#bazel_files[@]} -gt 0 ]] && format "Bazel" "buildifier -lint=fix" "${bazel_files[@]}"
[[ ${#toml_files[@]} -gt 0 ]] && format "TOML" "taplo fmt --config" "$TAPLO_CONFIG" "${toml_files[@]}"

echo "Waiting for formatters to finish..."
for pid in "${pids[@]}"; do
  wait "$pid"
done

echo "Formatting complete."
