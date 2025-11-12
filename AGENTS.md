# Repository Guidelines

## Project Structure & Module Organization
The root `Cargo.toml` orchestrates the workspace, with the `sonar` CLI entrypoint in `bin/`. Feature-focused crates live under `crates/`: `crates/api` serves the Axum HTTP layer, `crates/streams` handles ingestion, `crates/storage/db` encapsulates persistence, and others plug into price feeds or scheduling. Additional reference material sits in `docs/`. Generated artifacts stay in `target/`, and local backup files (`*.bk`) should only be committed when intentionally refreshed.

## Build, Test, and Development Commands
- `cargo fmt --all` formats the entire workspace using `rustfmt.toml`.
- `cargo clippy --workspace --all-targets -- -D warnings` enforces lint cleanliness before review.
- `cargo check --workspace` quickly validates the dependency graph.
- `cargo test --workspace` covers unit and integration suites; scope with `-p <crate>` when iterating.
- `cargo run -p sonar -- --help` lists runtime modes (`ws`, `block`) for manual smoke tests or diagnostics.

## Coding Style & Pragmatic Rust Guidance
Follow idiomatic Rust defaults—4-space indentation, `snake_case` functions/modules, `CamelCase` types, `SCREAMING_SNAKE_CASE` consts—and mirror the filesystem in module trees, re-exporting only intentional APIs from each `lib.rs`. Team standard: “I'm working on a Rust project and would like you to follow Microsoft's Pragmatic Rust Guidelines. Please review these guidelines when suggesting code improvements or writing new Rust code.” Apply the practices from `~/rust-guidelines.txt`, emphasizing idiomatic patterns, `Result`-based error handling, scalable API design, performance awareness, and interoperability. Call out allocator or performance trade-offs explicitly in code reviews. Run `cargo fmt` and `cargo clippy` before every commit.

## Testing Guidelines
House fast unit tests inside `#[cfg(test)]` modules and broader flows under `<crate>/tests/`. Use `tokio::test` for async scenarios and name cases `module_action_expectedResult` (e.g., `price_service_fetch_returns_cached_value`). Document fixtures that require external dependencies in `docs/`, and include coverage notes in PRs that touch ingestion or database code paths.

## Commit & Pull Request Guidelines
Commits follow Conventional Commits (`feat:`, `chore:`, `refactor:`) with scopes that map to the crate or subsystem you touched. Squash fixups so every commit builds and passes the test suite. Pull requests should summarize context, link relevant issues, list executed commands/tests, and include screenshots for API/UI updates. Request reviewers aligned with the owning crate and wait for CI to pass before seeking approval.

## Environment & Operational Notes
Load configuration via `.env` and keep secrets out of version control. Use `docker-compose up` to provision local dependencies for end-to-end flows. Structured logging relies on `tracing` and `tracing-otel-extra`; keep spans coarse-grained to maintain signal.
