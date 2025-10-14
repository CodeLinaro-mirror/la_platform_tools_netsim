# Copyright 2025 - The Android Open Source Project
"""nix bazel build rule"""

load("@rules_rust//rust:defs.bzl", "rust_library")

package(default_visibility = ["//visibility:public"])

rust_library(
    name = "nix",
    srcs = glob(["src/**/*.rs"]),
    crate_name = "nix",
    crate_root = "src/lib.rs",
    edition = "2021",
    features = [
        "event",
        "feature",
        "fs",
        "inotify",
        "ioctl",
        "memoffset",
        "mman",
        "mount",
        "net",
        "poll",
        "process",
        "sched",
        "signal",
        "socket",
        "term",
        "time",
        "uio",
        "user",
    ],
    rustc_flags = [
        "--cfg=feature=\"fs\"",
        "--cfg=linux",
        "--cfg=linux_android",
    ],
    deps = [
        "@bitflags",
        "@cfg-if",
        "@libc",
        #        "@memoffset",
    ],
)
