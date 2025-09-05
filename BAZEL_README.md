# Building and Testing with Bazel

This document provides instructions for building and testing the netsim project using Bazel.

## Building

To build a specific target, such as the `netsim` binary, use its label:

```bash
bazel build :netsim
```

The output binary will be located at `bazel-bin/netsim`.

## Testing

If you want to run a specific test suite, you can specify its target name. For example, to run the tests for `netsim_cli`:

```bash
bazel test :netsim_cli_tests
```

## Managing Dependencies

This project uses scripts to generate Bazel build files for external Rust dependencies from their original build system files (`Android.bp` or `Cargo.toml`).

### From Android.bp (android-crates-io)

To translate Rust crate dependencies from `Android.bp` files located in `external/rust/android-crates-io/crates`, use the `scripts/generate_deps.py` script.

**Usage:**

```bash
python3 scripts/generate_deps.py [options] <Android.bp_rule_name>...
```

**Example:**

To generate Bazel files for `liblog_rust` and `libfutures`:

```bash
python3 scripts/generate_deps.py liblog_rust libfutures
```

The generated `BUILD.bazel` files will be placed in the `bazel_deps/` directory.

NOTE: You must manually update `MODULE.bazel` to decalre a `new_local_repository` for any new crates introduced.

### From Cargo.toml (QEMU third-party)

To translate Rust crate dependencies from `Cargo.toml` files located in `external/qemu/android/third_party/rust/crates`, use the `scripts/generate_third_party_deps.py` script.

**Usage:**

```bash
python3 scripts/generate_third_party_deps.py [options] <CRATE_NAME>...
```

**Example:**

To generate Bazel files for the `argh` and `anyhow` crates:

```bash
python3 scripts/generate_third_party_deps.py argh anyhow
```

This script will generate the corresponding `BUILD.bazel` files in `bazel_deps/` with the new dependencies.

NOTE: You must manually update `MODULE.bazel` to decalre a `new_local_repository` for any new crates introduced.