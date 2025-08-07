# Contributing to Sonar

First off, thank you for considering contributing to Sonar! We welcome any help, from reporting a bug to submitting a feature request or a pull request.

## How Can I Contribute?

### Reporting Bugs

If you find a bug, please open an issue on our [GitHub Issues](https://github.com/sola-hq/sonar/issues) page.

Please include the following in your bug report:
- A clear and descriptive title.
- A detailed description of the problem, including steps to reproduce it.
- The expected behavior and what actually happened.
- Your environment details (e.g., OS, Rust version).

### Suggesting Enhancements

If you have an idea for a new feature or an improvement, please open an issue on our [GitHub Issues](https://github.com/sola-hq/sonar/issues) page. Please use a clear title and describe your idea in detail.

### Pull Requests

We love pull requests! If you'd like to contribute code, please follow these steps:

1.  **Fork the repository** on GitHub.
2.  **Clone your fork** locally:
    ```bash
    git clone https://github.com/sola-hq/sonar.git
    cd sonar
    ```
3.  **Create a new branch** for your changes:
    ```bash
    git checkout -b feature/your-feature-name
    ```
4.  **Make your changes**. Please ensure your code adheres to the project's style.
5.  **Run the linter and tests** to ensure everything is working correctly:
    ```bash
    cargo fmt --all --check
    cargo clippy --all -- -D warnings
    cargo test --all
    ```
6.  **Commit your changes** with a clear and descriptive commit message:
    ```bash
    git commit -m "feat: Add new feature that does X"
    ```
7.  **Push your branch** to your fork:
    ```bash
    git push origin feature/your-feature-name
    ```
8.  **Open a Pull Request** on the original repository. Please provide a clear description of the changes you've made.

## Development Setup

1.  Make sure you have the correct Rust toolchain installed (see `rust-toolchain.toml`).
2.  Install any other dependencies mentioned in the `README.md`.
3.  Build the project: `cargo build`
4.  Run the tests: `cargo test --all`

## Code Style

We use `rustfmt` for formatting and `clippy` for linting. Please run `cargo fmt --all` before committing your changes to ensure your code is formatted correctly.

Thank you for your contribution!
