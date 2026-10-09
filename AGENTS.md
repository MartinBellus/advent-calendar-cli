# Repository Guidelines

## Project Structure & Module Organization

This is a small Rust command-line application. `Cargo.toml` defines the package and dependencies; application code currently lives in `src/main.rs`. Keep implementation code in `src/`, integration tests in `tests/`, and project documentation or configuration at the repository root. Add assets only when the CLI needs them, and keep them in a clearly named directory.

## General Style

Do not use memories, write reccuring issues to AGENTS.md.

## Build, Test, and Development Commands

Run commands from the repository root with Cargo:

- `cargo check --offline` checks, if the code compiles
- `cargo run --offline` builds and launches the CLI.
- `cargo build --offline` compiles the project.
- `cargo test --offline` runs unit and integration tests.
- `cargo fmt --check` checks Rust formatting; use `cargo fmt` to apply it.
- `cargo clippy --all-targets` runs Rust lints when Clippy is installed.

## Coding Style & Naming Conventions

Use the Rust 2024 edition specified in `Cargo.toml` and standard `rustfmt` formatting (four-space indentation). Follow Rust naming conventions: `snake_case` for functions, variables, and modules; `UpperCamelCase` for types and traits; and `SCREAMING_SNAKE_CASE` for constants. Keep command behavior and parsing easy to follow, and split code into modules as the application grows.

## Testing Guidelines

Place focused unit tests beside the code they cover in `#[cfg(test)]` modules. Put end-to-end CLI checks in `tests/` when needed. Name tests for the behavior they verify, and cover argument handling, calendar calculations, and user-visible output as those features are added. Run `cargo test` after behavior changes.

## Commit & Pull Request Guidelines

- Keep the repository owner as the author; do not use `Codex` as the author name.
- Add `Co-authored-by: Codex <noreply@openai.com>` to commits created by this agent.
- Stage completed changes in focused, reviewable chunks and show the staged diff to the user.
- Do not commit until the user explicitly confirms. After confirmation, keep commits limited to related changes, use a concise imperative subject, and report the resulting commit.

## Configuration & Generated Files

Do not commit credentials or machine-specific settings. Keep generated build output such as `target/` out of version control, and document any required configuration in the repository documentation.
