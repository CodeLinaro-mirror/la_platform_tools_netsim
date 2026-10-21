@echo off
REM Copyright 2026 The Android Open Source Project
REM SPDX-License-Identifier: Apache-2.0
REM
REM Bazel credential helper used to fetch the GCS-hosted hermetic SDK archives
REM selected by --config=hermetic. See credential_helper.sh for why this thin
REM launcher exists instead of pointing --credential_helper at the shared
REM build/bazel/utils/credential_helper.cmd.
REM
REM %~dp0 is tools\netsim\scripts\, so ..\..\.. is the Android tree root.
"%~dp0..\..\..\prebuilts\python\windows-x86\python.exe" "%~dp0..\..\..\build\bazel\utils\cred_helper.py" %*
