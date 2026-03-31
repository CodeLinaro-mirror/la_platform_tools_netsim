// Copyright 2026 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

extern "C" void rust_main(int argc, char *argv[]);

int main(int argc, char *argv[]) { rust_main(argc, argv); }
