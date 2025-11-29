# Repository Guidelines
This is a small Rust CLI tool to analyze a directory containing source code and provide you detailed analysis of it.

## Project Structure & Module Organization
- `src/main.rs` wires the CLI entrypoint; `src/cli.rs` defines commands/flags; `src/analysis`, `src/display`, and `src/langs` hold the core logic for scanning, rendering, and language metadata; shared helpers sit in `src/utils.rs`.
- Templates for human output live in `templates/` (Askama), and language definitions live in `languages.json5`. Keep these in sync with any changes to output fields.
- Integration tests and fixtures are under `tests/`; add new fixture directories when expanding language coverage or output modes.

## Build, Test, and Development Commands
- `cargo +nightly fmt` — format the codebase with the repo settings.
- `cargo clippy -- -D warnings` — lint and fail on warnings.
- `cargo test` — run the integration and unit tests (uses fixtures in `tests/fixtures`).
- `cargo run -- analyze <path>` — manual check of analysis output; add `--output json|csv|markdown` to verify formats.
- `cargo build --release` — produce the optimized `cs` binary.

## Coding Style & Naming Conventions
- Rustfmt is authoritative (see `rustfmt.toml`): tabs for indentation, 120 column width, grouped imports by crate.
- Use idiomatic Rust naming: modules and functions `snake_case`, types and traits `PascalCase`, constants `SCREAMING_SNAKE_CASE`.
- Prefer small, single-purpose functions; keep CLI parsing in `cli.rs` and avoid mixing I/O with counting logic in analysis modules.

## Testing Guidelines
- Place new integration tests in `tests/` with descriptive file names (e.g., `fixture_counts.rs`-style). Keep fixtures minimal and committed.
- Use unit tests alongside modules for pure logic; prefer integration tests for end-to-end output checks (human, json, csv, markdown).
- When altering templates or language definitions, add/adjust tests to confirm formatting and totals.
