//  Copyright 2024 Google, Inc.
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at:
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
// TODO(b/203002625) - since rustc 1.53, bindgen causes UB warnings
// Remove this once bindgen figures out how to do this correctly
#![allow(deref_nullptr)]

#[cfg(target_os = "linux")]
include!("linux/bindings.rs");

#[cfg(target_os = "macos")]
include!("macos/bindings.rs");

#[cfg(target_os = "windows")]
include!("windows/bindings.rs");

extern "C" {
    pub fn run_hostapd_main(
        argc: ::std::os::raw::c_int,
        argv: *const *const std::os::raw::c_char,
    ) -> ::std::os::raw::c_int;
}
