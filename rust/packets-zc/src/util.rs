// Copyright 2025 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

//! Common utility types and functions for the `packets_zc` crate.

use zerocopy::Ref;

/// The result of a successful parse operation on a byte slice.
///
/// It contains a zero-copy reference to the parsed header (`T`) and a slice
/// representing the remaining bytes (`&'a [u8]`).
pub type ParseResult<'a, T> = (Ref<&'a [u8], T>, &'a [u8]);
