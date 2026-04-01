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

#!/usr/bin/env bash
#
# Bump the version.
#
# The CARGO_PKG_VERSION is not available for Android.bp builds
# so bump versions using a script.
#

# Absolute path to this script
SCRIPT=$(dirname $(readlink -f "$0"))
export CARGO=$SCRIPT/../rust/daemon/Cargo.toml
export CARGO_CLI=$SCRIPT/../rust/cli/Cargo.toml
export CARGO_COMMON=$SCRIPT/../rust/common/Cargo.toml
export VERSION=$SCRIPT/../rust/daemon/src/version.rs
export NEXT_DIR=$SCRIPT/../next
export NEXT_VERSION=$SCRIPT/../next/daemon/src/version.rs

VERSIONS=$(python <<EOF
import re
import os

m = None
new_version = ""
for cargo in [os.environ["CARGO_COMMON"], os.environ["CARGO_CLI"], os.environ["CARGO"]]:
    with open(cargo, "r+") as f:

        version_regex = re.compile(r'^version\s=\s"(\d+)\.(\d+)\.(\d+)"$')

        lines = f.readlines()
        for i, line in enumerate(lines):
            # Check if the line contains the string "version = "
            # and replace
            m = version_regex.match(line)
            if m:
                new_version = "{0}.{1}.{2}".format(m[1], m[2], int(m[3]) + 1)
                lines[i] = 'version = "{}"\n'.format(new_version)
                break

        f.seek(0)
        f.writelines(lines)
        f.truncate()

with open(os.environ["VERSION"], "r+") as f:
        lines = f.readlines()
        for i, line in enumerate(lines):
            if line.startswith("pub const VERSION"):
               lines[i] = 'pub const VERSION: &str = "{}";\n'.format(new_version)
               break

        f.seek(0)
        f.writelines(lines)
        f.truncate()

new_next_version = ""
if os.path.exists(os.environ["NEXT_VERSION"]):
    with open(os.environ["NEXT_VERSION"], "r+") as f:
        lines = f.readlines()
        next_ver_regex = re.compile(r'^pub const VERSION:\s&str\s=\s"(\d+)\.(\d+)\.(\d+)";$')
        for i, line in enumerate(lines):
            m = next_ver_regex.match(line)
            if m:
               new_next_version = "{0}.{1}.{2}".format(m[1], m[2], int(m[3]) + 1)
               lines[i] = 'pub const VERSION: &str = "{}";\n'.format(new_next_version)
               break
        f.seek(0)
        f.writelines(lines)
        f.truncate()

print(new_version + " " + new_next_version)
EOF
)

NEW_VERSION=$(echo $VERSIONS | awk '{print $1}')
NEW_NEXT_VERSION=$(echo $VERSIONS | awk '{print $2}')

echo "Bumping original to version $NEW_VERSION"
echo "Bumping next to version $NEW_NEXT_VERSION"

# Create a CL
cd "$SCRIPT/.."
repo start "version_bump_${NEW_VERSION}_${NEW_NEXT_VERSION}" .
git commit -m "Version Bump to $NEW_VERSION (next to $NEW_NEXT_VERSION)" "$CARGO" "$CARGO_CLI" "$CARGO_COMMON" "$VERSION" "$NEXT_VERSION"
repo upload -y --cbr -o nokeycheck --label Presubmit-Ready+1 --re=formosa@google.com,shuohsu@google.com --cc=schilit@google.com .
