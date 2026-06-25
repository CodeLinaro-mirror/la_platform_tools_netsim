#!/bin/bash
# Copyright 2022 The Android Open Source Project
#
# Formats source files according to Google's style guide.
# By default, formats all files.
# Use --diff to format files that are different from HEAD (including untracked).
# Use --hook to format files passed as arguments (e.g. for pre-commit).

# --- Bash Version Check (Strict 4.4+) ---
if ((BASH_VERSINFO[0] < 4 || (BASH_VERSINFO[0] == 4 && BASH_VERSINFO[1] < 4))); then
  echo "Error: Bash 4.4+ is required."
  # Try to find a suitable bash
  for p in /opt/homebrew/bin/bash /usr/local/bin/bash; do
    if [[ -x "$p" ]] && "$p" -c \
       '((BASH_VERSINFO[0] > 4 || (BASH_VERSINFO[0] == 4 && BASH_VERSINFO[1] >= 4)))'; then
       exec "$p" "$0" "$@"
    fi
  done
  exit 1
fi

set -euo pipefail

# --- Setup ---
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."
REPO="$SCRIPT_DIR/../../.."
OS=$(uname | tr '[:upper:]' '[:lower:]')
DESIRED_TAPLO_VERSION="0.10.0"

# --- Argument Parsing ---
MODE="ALL"
if [[ $# -gt 0 ]]; then
  case "$1" in
    --diff) MODE="DIFF"; shift ;;
    --hook) MODE="HOOK"; shift ;;
    *) echo "Unknown argument: $1"; exit 1 ;;
  esac
fi

# --- Configuration ---
RUSTFMT="$REPO/prebuilts/rust/$OS-x86/stable/rustfmt"
if [[ ! -f "$RUSTFMT" ]] && command -v rustfmt &> /dev/null; then
  RUSTFMT="rustfmt"
fi
BPFMT="$REPO/prebuilts/build-tools/$OS-x86/bin/bpfmt"
TAPLO_CONFIG="$REPO/tools/netsim/next/taplo.toml"

KTFMT="ktfmt"
if ! command -v ktfmt &> /dev/null; then
  if [ -x "/google/bin/releases/kotlin-google-eng/ktfmt/ktfmt" ]; then
    KTFMT="/google/bin/releases/kotlin-google-eng/ktfmt/ktfmt"
  fi
fi

# Common find exclusions
EXCLUDES=(
  -not -path '*/target/*'
  -not -path './.git/*'
  -not -path './bazel-out/*'
  -not -path './objs/*'
)

# Languages and their "Struct" configurations

# Clang
Clang_CMD="clang-format -i"
Clang_DIRS="src rust next proto ui/ts"
Clang_EXTS="-name *.cc -o -name *.h -o -name *.proto -o -name *.ts"
Clang_REGEX="\.(cc|h|proto|ts)$"

# Rust
Rust_CMD="$RUSTFMT --files-with-diff"
Rust_DIRS="rust next proto"
Rust_EXTS="-name *.rs"
Rust_REGEX="\.rs$"

# Java
Java_CMD="google-java-format -i"
Java_DIRS="."
Java_EXTS="-name *.java"
Java_REGEX="\.java$"

# Python
Python_CMD="pyformat --in_place --alsologtostderr --noshowprefixforinfo"
Python_DIRS="."
Python_EXTS="-name *.py"
Python_REGEX="\.py$"

# CMake
CMake_CMD="cmake-format -i"
CMake_DIRS="."
CMake_EXTS="-name CMakeLists.txt -o -name *.cmake"
CMake_REGEX="CMakeLists\.txt$|\.cmake$"

# Blueprint
Blueprint_CMD="$BPFMT -w"
Blueprint_DIRS="."
Blueprint_EXTS="-name Android.bp"
Blueprint_FLAGS="-maxdepth 1"
Blueprint_REGEX="Android\.bp$"

# Bazel
Bazel_CMD="buildifier -lint=fix"
Bazel_DIRS="."
Bazel_EXTS="-name BUILD -o -name MODULE.bazel \
-o -name BUILD.bazel -o -name *.bzl"
Bazel_REGEX="BUILD$|MODULE\.bazel$|BUILD\.bazel$|\.bzl$"

# Toml
Toml_CMD="env RUST_LOG=warn taplo fmt --config $TAPLO_CONFIG"
Toml_DIRS="rust next proto"
Toml_EXTS="-name Cargo.toml"
Toml_FLAGS="-not -path */bazel-bin/* -not -path */bazel-netsim/*"
Toml_REGEX="Cargo\.toml$"

# Kotlin
Kotlin_CMD="$KTFMT --google-style"
Kotlin_DIRS="."
Kotlin_EXTS="-name *.kt"
Kotlin_REGEX="\.kt$"

# License Headers
License_CMD="$SCRIPT_DIR/format_licenses.py"
License_DIRS="."
License_EXTS="-name *.rs -o -name *.toml -o -name BUILD* -o -name *.bzl -o -name *.cc -o -name *.h -o -name *.py -o -name *.java -o -name *.ts -o -name *.proto -o -name *.kt -o -name CMakeLists.txt"
License_REGEX="\.(rs|toml|bzl|cc|h|py|java|ts|proto|kt)$|^BUILD|^CMakeLists\.txt$"

# Ordered list of languages to process
LANGS=(License Clang Rust Java Python CMake Blueprint Bazel Toml Kotlin)

# --- Helpers ---

check_taplo_version() {
  if ! command -v taplo &> /dev/null; then
    echo "Error: 'taplo' not found. Please install taplo-cli."
    exit 1
  fi
  local INSTALLED_VERSION
  INSTALLED_VERSION=$(taplo --version | awk '{print $2}')
  if [ "$INSTALLED_VERSION" != "$DESIRED_TAPLO_VERSION" ]; then
    echo "Error: Found taplo version ${INSTALLED_VERSION}, but \
${DESIRED_TAPLO_VERSION} is required."
    echo "Please install correct version via cargo."
    exit 1
  fi
}

format_files() {
  local lang="$1"
  # Use nameref for struct access
  declare -n cmd_ref="${lang}_CMD"
  local -n files_ref="$2"

  if [[ ${#files_ref[@]} -eq 0 ]]; then
    return
  fi

  local executable
  executable=$(echo "${cmd_ref}" | awk '{print $1}')

  if ! command -v "$executable" &> /dev/null; then
    echo "Error: '$executable' not found, skipping $lang file formatting."
    return 0
  fi

  echo "Formatting ${#files_ref[@]} $lang files..."
  if [[ "${3:-}" == "SYNC" ]]; then
    printf "%s\0" "${files_ref[@]}" | xargs -0 -P 4 -n 100 ${cmd_ref}
  else
    printf "%s\0" "${files_ref[@]}" | xargs -0 -P 4 -n 100 ${cmd_ref} &
    pids+=($!)
  fi
}

check_taplo_version

# --- Regenerate Android.bp files ---
echo "Regenerating Android.bp files..."
python3 "$SCRIPT_DIR/bzl2bp.py"

# --- Main Logic ---

# Declare file arrays dynamically
for lang in "${LANGS[@]}"; do
  eval "FILES_$lang=()"
done

pids=()

if [[ "$MODE" == "ALL" ]]; then
  echo "Gathering all files to format..."
  for lang in "${LANGS[@]}"; do
    declare -n dirs_ref="${lang}_DIRS"
    declare -n exts_ref="${lang}_EXTS"
    declare -n flags_ref="${lang}_FLAGS"

    # Split regex/globs for find (SC2086 is desired here)
    # shellcheck disable=SC2086
    set -f
    find_args=( \( ${exts_ref} \) )
    set +f

    flags=()
    if [[ -n "${flags_ref:-}" ]]; then
       # shellcheck disable=SC2086
       flags=( ${flags_ref} )
    fi

    declare -n target_ref="FILES_$lang"
    # shellcheck disable=SC2086
    mapfile -d '' target_ref < <(find ${dirs_ref} "${flags[@]}" \
      -type f "${find_args[@]}" "${EXCLUDES[@]}" -print0)
  done

else
  echo "Gathering files to format..."
  all_candidates=()

  if [[ "$MODE" == "DIFF" ]]; then
    mapfile -d '' all_candidates < <(git diff -z --name-only \
      --diff-filter=ACMRTUXB HEAD && \
      git ls-files -z --others --exclude-standard)
  else # HOOK
    all_candidates=("$@")
  fi

  if [ ${#all_candidates[@]} -eq 0 ]; then
    echo "No files to format."
    exit 0
  fi

  for f in "${all_candidates[@]}"; do
    for lang in "${LANGS[@]}"; do
       declare -n regex_ref="${lang}_REGEX"
       if [[ "$f" =~ ${regex_ref} ]]; then
          declare -n target_ref="FILES_$lang"
          target_ref+=("$f")
       fi
    done
  done
fi

# --- Execution ---

for lang in "${LANGS[@]}"; do
  if [[ "$lang" == "License" ]]; then
    format_files "$lang" "FILES_$lang" "SYNC"
  else
    format_files "$lang" "FILES_$lang"
  fi
done

# Wait for background processes and check for failures
FAIL=0
if [[ ${#pids[@]} -gt 0 ]]; then
  echo "Waiting for formatters to finish..."
  for pid in "${pids[@]}"; do
    if ! wait "$pid"; then
      FAIL=1
    fi
  done
fi

if [[ $FAIL -ne 0 ]]; then
  echo "Formatting failed for some files."
  exit 1
fi

echo "Formatting complete."
