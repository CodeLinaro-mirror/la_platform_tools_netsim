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

