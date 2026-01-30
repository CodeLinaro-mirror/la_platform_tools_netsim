"""
Netsim Rust build rules.

This module provides the netsim_rust_library macro which wraps rust_library
to provide standard targets for testing, linting, and formatting.
"""

load("@rules_rust//rust:defs.bzl", "rust_clippy", "rust_common", "rust_doc", "rust_doc_test", "rust_library", "rust_test")

NETSIM_RUSTC_FLAGS = ["-Dwarnings"]

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

def netsim_rust_library(
        name,
        srcs,
        deps = [],
        select_deps = [],
        test_deps = [],
        crate_features = [],
        # Feature Flags (Default to True for safety)
        enable_clippy = True,
        enable_rustfmt = True,
        enable_unit_test = True,
        enable_doc = True,
        enable_doc_test = True,
        strict_warnings = True,
        **kwargs):
    """
    Defines a Netsim Rust library with standard targets.

    This macro wraps rust_library and automatically generates:
    - _test: Unit tests for the crate.
    - _clippy: Clippy checks.
    - _fmt: Rustfmt checks.
    - _doc: Documentation generation.
    - _doc_test: Documentation tests.

    Args:
        name: The name of the library.
        srcs: The source files.
        deps: The dependencies.
        select_deps: The part of deps that uses select()
        test_deps: Dependencies for the unit test target.
        crate_features: the crate features
        enable_clippy: Whether to enable clippy checks.
        enable_rustfmt: Whether to enable rustfmt checks.
        enable_unit_test: Whether to generate a unit test target.
        enable_doc: Whether to generate documentation.
        enable_doc_test: Whether to generate a doc test target.
        strict_warnings: Whether to enforce strict warnings (treat warnings as errors).
        **kwargs: Additional arguments passed to rust_library.
    """

    # 1. The Main Library (Always created)
    rustc_flags = kwargs.pop("rustc_flags", [])
    if strict_warnings:
        rustc_flags = NETSIM_RUSTC_FLAGS + rustc_flags

    rust_library(
        name = name,
        srcs = srcs,
        deps = deps + select_deps,
        rustc_flags = rustc_flags,
        crate_features = crate_features,
        **kwargs
    )

    # 2. The Testing Library with "testing" feature (always created)
    # All deps are //next/crate -> //next/crate:testing
    testing_deps = [
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

    # 3. Automatic Unit Test (againt testing library)
    if enable_unit_test:
        test_flags = []
        if strict_warnings:
            test_flags = NETSIM_RUSTC_FLAGS

        rust_test(
            name = "test",
            crate = ":testing",
            rustc_flags = test_flags,
            deps = testing_deps + select_deps + test_deps,
            testonly = True,
        )

    # 4. Clippy Linter
    if enable_clippy:
        rust_clippy(
            name = "clippy",
            deps = [":" + name],
            testonly = True,
        )

    # 5. Rustfmt Check
    if enable_rustfmt:
        netsim_rustfmt_test(
            name = "rustfmt",
            targets = [":" + name],
            # Restrict to Linux for CI validation.
            # Note: this fails on macOS because the librustc_driver dylib isn't available
            target_compatible_with = ["@platforms//os:linux"],
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
        # Restrict to Linux due to macOS toolchain issues:
        # 1. The macOS `goldfish_build+` toolchain's C++ linker wrapper is missing from the sandbox for pure Rust targets.
        # 2. Mixed C++ crates work (they pull the toolchain in), but pure Rust crates fail.
        # 3. Linux provides sufficient CI validation.
        rust_doc_test(
            name = "doc-test",
            crate = ":" + name,
            testonly = True,
            target_compatible_with = ["@platforms//os:linux"],
        )
