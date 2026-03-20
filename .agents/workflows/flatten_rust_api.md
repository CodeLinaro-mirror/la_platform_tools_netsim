---
description: How to flatten a deeply nested Rust API using the Facade pattern
---

# Flattening a Rust API with a Facade

This workflow provides a structured methodology for encapsulating internal modules and flattening a Rust library's public API. This is particularly useful for library crates (e.g., `netsim` actors) that have evolved iteratively with deeply nested structures (e.g., `crate::network::client::Client`) and need cleanup to hide internal implementation details.

## Objectives
1. **Encapsulate Implementation**: Hide internal module hierarchies from the public documentation and crate interface.
2. **Flatten via Re-exports**: Lift essential components to the crate root.
3. **Restrict Visibility**: Demote strictly internal constructs to `pub(crate)` or `pub(super)`.
4. **Preserve Logic**: Do not alter implementation logic or function signatures.

> [!NOTE]
> By flattening the API, we ensure consumers only need to import from the root module (e.g., `use my_crate::Client;` instead of `use my_crate::network::client::Client;`), making the library much easier to use, document, and maintain.

## Systematic Workflow

### Phase 1: Assess the Public API

1. Inspect the entry point (`lib.rs` or the main module). Identify all currently exposed `pub mod <name>;` declarations.
2. Search through the crate for key data structures (`struct`, `enum`, `trait`, essential `fn` and `type` definitions) that are naturally intended to be public.
3. Plan the explicit re-exports to be placed directly at the crate root.

> [!TIP]
> **Avoid wildcard exports like `pub use crate::module::*;`**. While convenient, wildcard re-exports pollute the public namespace and obscure the origin of symbols. Explicit re-exports (`pub use crate::module::Type;`) are strongly preferred for a robust, maintainable codebase.

### Phase 2: Refactoring Module Structure

1. **Update `lib.rs`**: Change all `pub mod <name>;` to `mod <name>;` (or `pub(crate) mod <name>;`).
2. **Construct the Facade**: Re-export the essential types at the root of `lib.rs` using the `pub use` pattern.

**Example `lib.rs` Before:**
```rust
pub mod network;
pub mod client;
```

**Example `lib.rs` After:**
```rust
// Modules are now strictly private to the crate
mod network;
mod client;

// The Facade: explicit, intentional surface area
pub use network::Connection;
pub use client::{Client, ClientError};

// Backwards compatibility for external consumers (if explicitly required)
#[deprecated(since = "1.2.0", note = "Import Connection directly from crate root")]
pub mod network {
    pub use super::Connection;
}
```

### Phase 3: Demoting Visibility

1. Iterate over the internal modules. Change types (`struct`, `enum`, `fn`) that are no longer part of the public facade from `pub` to `pub(crate)`.
2. Keep in mind that modules are purely private now; however, explicit `pub(crate)` effectively communicates intent to future developers and clearly demarcates the crate's internal boundaries.

> [!IMPORTANT]
> Any type exposed via the facade in Step 2 **must remain** `pub` in its defining module. If you change it to `pub(crate)`, rustc will generate a "cannot re-export private type" error.

### Phase 4: Resolving "Private Type in Public Interface" Errors

1. Build the target crate using `scripts/build_tools.py --task compile`.
2. Hiding types will inevitably surface `[E0446] private type in public interface` compiler errors.
3. Rust enforces that if a `pub` struct or function takes/returns a parameter, that parameter type must also be publicly visible.

> [!WARNING]
> Resolve these errors by tracking the dependencies reported by rustc. You must either:
> 1. Promote those missing internal types back to `pub` and add them to the `lib.rs` facade.
> 2. Change the offending public interface down to `pub(crate)` if it doesn't strictly need to be public.
> 3. Use `#[doc(hidden)]` on the type if it *must* remain structurally public (e.g. for macros or traits) but shouldn't clutter the generated API documentation.

### Phase 5: Testing and Integration Fixes

1. Hiding internal paths means that external consumers relying on deep paths (e.g., `use my_crate::internal_mod::MyStruct;`) will now fail to compile.
2. Run a workspace-wide build using `scripts/build_tools.py --task compile`.
3. Locate deep path usages natively inside sister crates, `tests/` integration directories, and consuming binaries, and update them.
4. **Use IDE refactoring tools**: Avoid using `sed`, bash scripts, or naive regex find/replace. Rely on `rust-analyzer`'s native Rename functionality (`F2`) or semantic refactoring to carefully fix these path breakages workspace-wide without unintended string replacements.
5. Guarantee semantic correctness by verifying the test suite using `scripts/build_tools.py --task test`.

## Reminders

> [!CAUTION]
> Remember that `tests/` directories represent *external* integration crates. Crate types utilized in integration tests must remain `pub` and exported via the facade.

Making a type `pub(crate)` will correctly trigger dead code warnings if it is unused within its crate. If the type is legitimately unused but shouldn't be deleted per constraints, `#[allow(dead_code)]` may be carefully applied to the specific item.
