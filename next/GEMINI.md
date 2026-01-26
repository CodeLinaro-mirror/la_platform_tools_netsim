---
trigger: always_on
---

# Build Instructions

> [!WARNING]
> Do not use cargo.

To build all next components, use the following command:
`./scripts/build_tools.py --task bazel`

To build individual components, use the following command:
`bazel build @netsim//next/<component_name>`
Example: `bazel build @netsim//next/wifi-actor`

# Testing Instructions

To run tests for individual components, use the following command:
`bazel test @netsim//next/<component_name>:integration-test`
Example: `bazel test @netsim//next/ap-actor:integration-test`

# Testing Flakes

To look for flakes, use the following Bazel command:
`bazel test @netsim//next/<component_name>:integration-test --test_env=RUST_LOG=debug --nocache_test_results --runs_per_test=100`
Example: `bazel test @netsim//next/ap-actor:integration-test --test_env=RUST_LOG=debug --nocache_test_results --runs_per_test=100`
