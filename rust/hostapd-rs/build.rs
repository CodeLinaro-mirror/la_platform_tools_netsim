// Copyright 2024 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub fn main() {
    // Shared dependencies
    println!("cargo:rustc-link-search=../objs/archives");
    println!("cargo:rustc-link-lib=hostapd");
    println!("cargo:rustc-link-lib=crypto");
    println!("cargo:rustc-link-lib=android-emu-base");
    println!("cargo:rustc-link-lib=android-emu-utils");
    println!("cargo:rustc-link-lib=logging-base");
    println!("cargo:rustc-link-lib=android-emu-base-logging");
    // Linux and Mac dependencies
    #[cfg(unix)]
    {
        println!("cargo:rustc-link-search=../objs/lib64");
        println!("cargo:rustc-link-lib=c++");
    }
    // Windows dependencies
    #[cfg(windows)]
    {
        println!("cargo:rustc-link-lib=crypto_asm_lib");
        println!("cargo:rustc-link-search=../objs/msvc-posix-compat/msvc-compat-layer");
        println!("cargo:rustc-link-lib=msvc-posix-compat");
    }
}
