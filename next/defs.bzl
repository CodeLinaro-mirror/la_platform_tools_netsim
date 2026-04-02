# Copyright 2026 The Android Open Source Project
# SPDX-License-Identifier: Apache-2.0

"""
Netsim Rust build rules.

This module provides the netsim_rust_library macro which wraps rust_library
to provide standard targets for testing, linting, and formatting.
"""

load("@rules_rust//rust:defs.bzl", "rust_binary", "rust_clippy", "rust_common", "rust_doc", "rust_doc_test", "rust_library", "rust_test")

NETSIM_RUSTC_FLAGS = ["-Dwarnings", "-Dunused_crate_dependencies"]

# Unfortunately, we can't use the rules_rust version because netsim is in external/.
def _netsim_rustfmt_test_impl(ctx):
    toolchain = ctx.toolchains["@rules_rust//rust:toolchain_type"]
    rustfmt, config = toolchain.rustfmt, ctx.file.config

    srcs = []
    for t in ctx.attr.targets:
        info = t[rust_common.crate_info] if rust_common.crate_info in t else getattr(t, "crate_info", None)
        if info:
            srcs.extend([s for s in info.srcs.to_list() if s.is_source])

    if not srcs:
        fail("No sources to format")

    runner = ctx.actions.declare_file(ctx.label.name + ".sh")
    ctx.actions.write(
        output = runner,
        is_executable = True,
        content = "#!/bin/bash\n%s --check --config-path %s %s" % (
            rustfmt.short_path,
            config.short_path,
            " ".join([s.short_path for s in srcs]),
        ),
    )

    return [DefaultInfo(
        executable = runner,
        runfiles = ctx.runfiles(files = srcs + [rustfmt, config] + toolchain.all_files.to_list()),
    )]

netsim_rustfmt_test = rule(
    implementation = _netsim_rustfmt_test_impl,
    attrs = {
        "targets": attr.label_list(providers = [[rust_common.crate_info]]),
        "config": attr.label(allow_single_file = True, default = Label("//next:rustfmt.toml")),
    },
    test = True,
    toolchains = ["@rules_rust//rust:toolchain_type"],
)

def _define_common_targets(name, enable_clippy, enable_rustfmt, targets = [], deps = []):
    # Clippy Linter
    if enable_clippy:
        rust_clippy(
            name = name + "_clippy",
            deps = deps,
            testonly = True,
        )

    # Rustfmt Check
    if enable_rustfmt:
        netsim_rustfmt_test(
            name = name + "_rustfmt",
            targets = targets,
            # Restrict to Linux for CI validation.
            target_compatible_with = ["@platforms//os:linux"],
        )

def netsim_rust_library(
        name,
        srcs,
        deps = [],
        select_deps = [],
        testing_deps = [],
        test_deps = [],
        crate_features = [],
        # Feature Flags (Default to True for safety)
        enable_clippy = True,
        enable_rustfmt = True,
        enable_unit_test = True,
        enable_integration_test = True,
        enable_doc = True,
        enable_doc_test = True,
        **kwargs):
    """
    Defines a Netsim Rust library with standard targets.

    This macro wraps rust_library and automatically generates:
    - _test: Unit tests for the crate.
    - _integration_test: Integration tests for the crate if tests/ subdirectory has files.
    - _clippy: Clippy checks.
    - _fmt: Rustfmt checks.
    - _doc: Documentation generation.
    - _doc_test: Documentation tests.

    Args:
        name: The name of the library.
        srcs: The source files.
        deps: The dependencies.
        select_deps: The part of deps that uses select()
        testing_deps: Dependencies for the "testing" feature.
        test_deps: Dependencies for the unit test and integration test targets.
        crate_features: the crate features
        enable_clippy: Whether to enable clippy checks.
        enable_rustfmt: Whether to enable rustfmt checks.
        enable_unit_test: Whether to generate a unit test target.
        enable_integration_test: Whether to generate an integration test target if files exist.
        enable_doc: Whether to generate documentation.
        enable_doc_test: Whether to generate a doc test target.
        **kwargs: Additional arguments passed to rust_library.
    """

    # 1. The Main Library (Always created)
    rustc_flags = NETSIM_RUSTC_FLAGS + kwargs.pop("rustc_flags", [])

    rust_library(
        name = name,
        srcs = srcs,
        deps = deps + select_deps,
        rustc_flags = rustc_flags,
        crate_features = crate_features,
        **kwargs
    )

    # 2. The Testing Library with "testing" feature (always created)
    testing_deps = testing_deps + [
        d + ":testing" if d.startswith("//next") else d
        for d in deps
    ]
    rust_library(
        name = "testing",
        srcs = srcs,
        deps = testing_deps + select_deps,
        rustc_flags = rustc_flags,
        crate_features = ["testing"] + crate_features,
        **kwargs
    )

    # 3. Automatic Unit Test
    if enable_unit_test:
        # There can be integ test deps that are not used in unit tests,
        # so disable unused_crate_dependencies.
        test_flags = [f for f in NETSIM_RUSTC_FLAGS if f != "-Dunused_crate_dependencies"]

        rust_test(
            name = "test",
            crate = ":testing",
            crate_features = ["testing"] + crate_features,
            rustc_flags = test_flags,
            deps = testing_deps + select_deps + test_deps,
            testonly = True,
        )

    # 4. Automatic Integration Test
    integration_test_srcs = native.glob(["tests/**/*.rs"], allow_empty = True)
    has_integration_test = enable_integration_test and len(integration_test_srcs) > 0

    if has_integration_test:
        # Integration tests receive all library dependencies for convenience,
        # so disable unused_crate_dependencies.
        test_flags = [f for f in NETSIM_RUSTC_FLAGS if f != "-Dunused_crate_dependencies"]

        # If there's a mod.rs, it's typically the root.
        # Else it's likely a single-file setup.
        crate_root_opts = [
            "tests/mod.rs",
            "tests/integration_tests.rs",
        ]
        crate_root = None
        for opt in crate_root_opts:
            if opt in integration_test_srcs:
                crate_root = opt
                break

        rust_test(
            name = "integration-test",
            srcs = integration_test_srcs,
            crate_root = crate_root,
            rustc_flags = test_flags,
            compile_data = kwargs.get("compile_data", []),
            deps = [":testing"] + testing_deps + select_deps + test_deps,
            proc_macro_deps = kwargs.get("proc_macro_deps", []),
            edition = kwargs.get("edition", "2021"),
            testonly = True,
        )

    # 5. Common Targets (Clippy, Rustfmt)
    _define_common_targets(
        name,
        enable_clippy,
        enable_rustfmt,
        targets = [":" + name] + (([":integration-test"] if has_integration_test else [])),
        deps = [":" + name] + (([":integration-test"] if has_integration_test else [])),
    )

    # 6. Documentation
    if enable_doc:
        rust_doc(
            name = "doc",
            crate = ":" + name,
            testonly = True,
        )

    # 7. Documentation Test
    if enable_doc_test:
        rust_doc_test(
            name = "doc-test",
            crate = ":" + name,
            testonly = True,
            target_compatible_with = ["@platforms//os:linux"],
        )

def netsim_rust_binary(
        name,
        srcs,
        deps = [],
        select_deps = [],
        crate_features = [],
        # Feature Flags (Default to True for safety)
        enable_clippy = True,
        enable_rustfmt = True,
        **kwargs):
    """
    Defines a Netsim Rust binary with standard targets.

    This macro wraps rust_binary and automatically generates:
    - _clippy: Clippy checks.
    - _fmt: Rustfmt checks.

    Args:
        name: The name of the binary.
        srcs: The source files.
        deps: The dependencies.
        select_deps: The part of deps that uses select()
        crate_features: the crate features
        enable_clippy: Whether to enable clippy checks.
        enable_rustfmt: Whether to enable rustfmt checks.
        **kwargs: Additional arguments passed to rust_binary.
    """

    # 1. The Main Binary
    rustc_flags = NETSIM_RUSTC_FLAGS + kwargs.pop("rustc_flags", [])

    rust_binary(
        name = name,
        srcs = srcs,
        deps = deps + select_deps,
        rustc_flags = rustc_flags,
        crate_features = crate_features,
        **kwargs
    )

    # 2. Common Targets (Clippy, Rustfmt)
    _define_common_targets(name, enable_clippy, enable_rustfmt, targets = [":" + name], deps = [":" + name])
