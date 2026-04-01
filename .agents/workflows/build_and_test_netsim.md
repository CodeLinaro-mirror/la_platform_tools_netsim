---
description: How to build and test the netsim-next project using the official scripts and Bazel
---

# Building and Testing Netsim Next

This workflow guides you through compiling, testing, and debugging the `netsim`
project. It prioritizes the official `build_tools.py` script as well as direct
Bazel invocations for fine-grained development iteration.

> [!WARNING]
> Do not use `cargo test` or `cargo build` directly. The project relies on
> Bazel for hermetic compilation and platform-specific macro resolution.

## 1. Workspace-wide Build and Test

The standard, most reliable method to verify the entire workspace in one go is
to use the provided builder script.

To build and run all tests for the `next` architecture (including compilation
checks):
```bash
./scripts/build_tools.py --task build test
```

> [!TIP]
> The `build_tools.py` script ensures that the correct Bazel flags and caching
> environments are applied. Passing multiple tasks like `--task build test`
> sequences them correctly.

## 2. Iterative Component Development (Bazel)

When focusing on a specific `netsim` actor or module, direct Bazel invocations
provide faster feedback.

### Building Specific Components

To build an individual component (e.g. `wifi-actor`):
```bash
bazel build @netsim//next/wifi-actor
```
*Change `wifi-actor` to your target directory like `ap-actor` or `packets`.*

### Running Specific Tests

To run the integration suite or unit tests for an individual component:
```bash
bazel test @netsim//next/ap-actor:integration-test
```
You can also use the `...` wildcard to run all tests inside a crate:
```bash
bazel test @netsim//next/ap-actor/...
```

## 3. Advanced Diagnostic Scenarios

### Troubleshooting Flaky Tests

If an integration test (like `daemon`'s connection timeouts) is failing
sporadically, use Bazel's stress-testing flags to hunt down the flake log:

```bash
bazel test @netsim//next/<component_name>:integration-test \
    --test_env=RUST_LOG=debug \
    --nocache_test_results \
    --runs_per_test=100
```
* `--test_env=RUST_LOG=debug`: Exposes verbose console logging.
* `--nocache_test_results`: Forces Bazel to re-evaluate the test.
* `--runs_per_test=100`: Repeatedly runs the test to uncover race conditions.

### Code Formatting

If `packets_rustfmt` or any other formatting check fails during a run-test
phase, execute the global formatting script to automatically align the
codebase with our style standards:
```bash
./scripts/format_code.sh
```
