// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::empty_line_after_doc_comments)]

/// Version library.

const VERSION: &str = "1.0.27";

pub(crate) fn get_version() -> String {
    format!("{VERSION}{}", option_env!("NETSIM_VERSION_SUFFIX").unwrap_or_default())
}

pub(crate) fn strip_version_suffix(version: &str) -> &str {
    version.split_once('+').map_or(version, |(v, _)| v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_version_suffix() {
        assert_eq!(strip_version_suffix("1.0.26+15953120"), "1.0.26");
        assert_eq!(strip_version_suffix("1.0.26-dev+34fa5eff"), "1.0.26-dev");
    }
}
