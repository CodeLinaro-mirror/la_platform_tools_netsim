# netsim - a network simulation tool for multi-device use cases.

Netsim is a development stage open-source tool for testing and analysis of
android multi-device apps and frameworks. It offers radio level control and HCI
tracing.

## Development Setup

To ensure consistent code style and quality, this project uses a pre-commit hook to automatically format code. Please follow these steps once to set it up:

1.  **Install pipx (if you don't have it):**

    **On Linux:**
    ```bash
    sudo apt update
    sudo apt install pipx
    ```

    **On Mac:**
    ```bash
    brew install pipx
    ```
    Homebrew install instructions: go/homebrew.

2.  **Add pipx to your PATH:**
    ```bash
    pipx ensurepath # You may need to restart your shell after this.
    ```

3.  **Install pre-commit using pipx:**
    ```bash
    pipx install pre-commit
    ```

4.  **Install the Git hooks for this repository:**
    ```bash
    pre-commit install
    ```

After this one-time setup, the formatting script will run automatically before each commit.

## Known Issues

### `rust_doc_test` on macOS
`rust_doc_test` targets are currently disabled on macOS for pure Rust crates (e.g., `actor-framework`, `device-actor`).
*   **Cause:** The `goldfish_build+` toolchain on macOS uses a C++ linker wrapper that is not included in the Bazel sandbox for pure Rust targets.
*   **Workaround:** These tests are marked with `target_compatible_with = ["@platforms//os:linux"]` to skip them on macOS.
*   **Exception:** Crates with C++ dependencies (like `bluetooth` -> `rootcanal`) work correctly because they pull the C++ toolchain into the sandbox.
