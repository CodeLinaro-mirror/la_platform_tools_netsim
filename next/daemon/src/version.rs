// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::empty_line_after_doc_comments)]

/// Version library.

const VERSION: &str = "1.0.26";

pub(crate) fn get_version() -> String {
    VERSION.to_owned()
}
