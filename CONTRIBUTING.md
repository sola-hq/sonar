# Contributing to Sonar

First off, thank you for considering contributing to Sonar! We welcome contributions of all kinds, from bug reports and documentation improvements to new features and protocol integrations.

This document provides guidelines to help make the contribution process smooth and effective for everyone.

## How Can I Contribute?

- **Reporting Bugs**: If you find a bug, please open an issue and provide as much detail as possible, including steps to reproduce it.
- **Suggesting Enhancements**: If you have an idea for a new feature or an improvement to an existing one, open an issue to start a discussion.
- **Improving Documentation**: If you find parts of the documentation unclear or incomplete, feel free to submit a pull request with your improvements.
- **Writing Code**: If you want to fix a bug or implement a new feature, we'd love your help! We recommend starting with issues tagged `good first issue` or `help wanted`.

## Setting Up Your Development Environment

1.  **Fork & Clone**: Fork the repository to your own GitHub account and then clone it to your local machine.

    ```bash
    git clone https://github.com/YOUR_USERNAME/sonar.git
    cd sonar
    ```

2.  **Add Upstream Remote**: This will help you keep your fork in sync with the main repository.

    ```bash
    git remote add upstream https://github.com/sola-hq/sonar.git
    ```

3.  **Install Dependencies**: Ensure you have the correct Rust toolchain installed (see `rust-toolchain.toml`) and Docker for running services like Redis and ClickHouse.

4.  **Build the Project**: Build the project to make sure everything is set up correctly.

    ```bash
    cargo build --workspace
    ```

## Making Changes

1.  **Create a New Branch**: Always create a new branch for your changes. This makes the review process cleaner.

    ```bash
    git checkout -b feature/my-awesome-feature
    ```

2.  **Write Your Code**: Make your changes, and please adhere to the existing code style.

3.  **Code Style & Linting**: We use `rustfmt` for formatting and `clippy` for linting. Before committing, please run:

    ```bash
    cargo fmt --all
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    ```

4.  **Add Tests**: If you are adding a new feature or fixing a bug, please add a corresponding test case to prevent future regressions.

5.  **Commit Your Changes**: Use a clear and descriptive commit message.

    ```bash
    git commit -m "feat: Add support for My Awesome Feature"
    ```

## Submitting a Pull Request

1.  **Push Your Branch**: Push your feature branch to your fork.

    ```bash
    git push origin feature/my-awesome-feature
    ```

2.  **Open a Pull Request**: Go to the Sonar repository on GitHub and open a new Pull Request. Provide a clear description of the changes you have made.

3.  **Code Review**: One of the project maintainers will review your code. We may ask for some changes before merging. We aim to be responsive and helpful during this process.

Thank you again for your interest in contributing to Sonar! We look forward to your contributions.