// Copyright 2023 The Android Open Source Project
// SPDX-License-Identifier: Apache-2.0

use std::ffi::*;
use std::mem::MaybeUninit;

extern "C" {
    fn SHA1(d: *const c_uchar, n: usize, md: *mut c_uchar) -> *mut c_uchar;
}

/// FFI to C SHA1 utility for websocket accept
pub fn sha1(data: &[u8]) -> [u8; 20] {
    // Safety: data is valid bytes
    unsafe {
        let mut hash = MaybeUninit::<[u8; 20]>::uninit();
        SHA1(data.as_ptr(), data.len(), hash.as_mut_ptr() as *mut _);
        hash.assume_init()
    }
}
