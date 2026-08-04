// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::empty_line_after_doc_comments)]

/// Version library.

pub const VERSION: &str = "1.0.18";

pub fn get_version() -> String {
    VERSION.to_owned()
}
