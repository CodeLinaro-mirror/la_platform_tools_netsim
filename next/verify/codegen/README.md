# Verify Codegen

The `codegen` tool is a build-time utility that scans source files for BDD step
definitions and generates the necessary glue code to register them with the
`verify/lib` runtime.

## Supported Languages

- **Rust** (`.rs`): Generates Rust code that implements `AsyncStep` adapters and
  a `register_steps` function.
- **Kotlin** (`.kt`): Generates Kotlin code that registers step lambdas with a
  `StepRegistry`.

## Annotations

### Rust

Mark async functions or methods with `/// STEP: <Keywords> <Pattern>`.

```rust
/// STEP: Given I have a calculator
async fn reset_calculator(w: &mut World) { ... }

/// STEP: When I add (\d+)
async fn add(w: &mut World, val: i32) { ... }
```

**Supported Arguments:**

- `String`: Matches `(.*)` or similar string captures.
- `i32`, `f64`, etc.: Parsed from string captures using `FromStr`.
- `DataTable`: Injected from `StepContext.table`.

**Implicit Arguments:**

- `&mut World` (First argument for free functions).
- `&self`, `&mut self` (First argument for methods).

### Kotlin

Mark functions with `/// STEP: <Pattern>`.

```kotlin
// STEP: I have a calculator
fun resetCalculator(context: Context) { ... }
```

## Usage

This tool is typically invoked by Bazel rules (e.g., `rust_library` with a
`genrule` or custom macro).

```bash
verify_codegen <input_file> <output_file>
```
