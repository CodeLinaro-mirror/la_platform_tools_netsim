"""
Netsim Rust build rules.

This module provides the netsim_rust_library macro which wraps rust_library
to provide standard targets for testing, linting, and formatting.
"""

load("@rules_rust//rust:defs.bzl", "rust_clippy", "rust_doc", "rust_doc_test", "rust_library", "rust_test")

NETSIM_RUSTC_FLAGS = ["-Dwarnings"]

def netsim_rust_library(
        name,
        srcs,
        deps = [],
        # Feature Flags (Default to True for safety)
        enable_clippy = True,
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
        enable_clippy: Whether to enable clippy checks.

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
        deps = deps,
        rustc_flags = rustc_flags,
        **kwargs
    )

    # 2. Automatic Unit Test
    if enable_unit_test:
        test_flags = []
        if strict_warnings:
            test_flags = NETSIM_RUSTC_FLAGS

        rust_test(
            name = name + "_test",
            crate = ":" + name,
            rustc_flags = test_flags,
            testonly = True,
        )

    # 3. Clippy Linter
    if enable_clippy:
        rust_clippy(
            name = name + "_clippy",
            deps = [":" + name],
            testonly = True,
        )

    # 5. Documentation
    if enable_doc:
        rust_doc(
            name = name + "_doc",
            crate = ":" + name,
            testonly = True,
        )

    # 6. Documentation Test
    if enable_doc_test:
        rust_doc_test(
            name = name + "_doc_test",
            crate = ":" + name,
            testonly = True,
        )
