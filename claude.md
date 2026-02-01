# codestats

High-performance CLI tool (`cs`) for analyzing code statistics across 460+ programming languages. Counts files, lines, and bytes by language while respecting `.gitignore` and offering flexible output formats.

## Quick Reference

```bash
cargo build --release    # Build optimized binary
cargo test               # Run fixture-based integration tests
cargo clippy             # Lint (all warnings are errors)
cargo fmt                # Format code
```

## Architecture

The analysis pipeline follows these stages:

1. **File Discovery** (`analyzer.rs`) - Parallel gitignore-aware tree walking via `ignore` crate
2. **I/O Strategy** (`file_io.rs`) - Buffered for small files, mmap for files ≥256KB
3. **Encoding Detection** (`encoding.rs`) - UTF-8/UTF-16 detection, binary file filtering
4. **Line Classification** (`line_classifier.rs`) - Code/comment/blank/shebang categorization
5. **Statistics Aggregation** (`stats.rs`) - Thread-local accumulation with merge-on-drop

### Key Modules

| Module | Purpose |
|--------|---------|
| `src/cli.rs` | CLI argument definitions (clap derive) |
| `src/config.rs` | TOML config loading and CLI merging |
| `src/analysis/` | Core analysis pipeline |
| `src/langs/` | Language detection and data |
| `src/display/` | Output formatters (JSON, CSV, HTML, Markdown, etc.) |

## Code Style

### Formatting (rustfmt.toml)

- **Tabs** for indentation (not spaces)
- **120 character** line limit
- **Group imports**: std → external → crate
- **Merge derives**: combine multiple `#[derive()]` attributes

### Linting

All clippy warnings are errors. The codebase enables:

```rust
#![warn(clippy::all, clippy::cargo, clippy::nursery, clippy::pedantic, clippy::perf)]
#![deny(warnings)]
```

### Conventions

- Use `anyhow::Result<T>` for error propagation
- Use `#[must_use]` on constructors returning important values
- Use `#[inline]` sparingly, only for hot paths
- Prefer thread-local processing with merge-on-drop over shared locks
- Document complex pipelines at module level

## Language Data

Languages are defined in `languages.json5` and compiled to `src/langs/data.rs` at build time.

To add a language:
1. Add entry to `languages.json5` (maintain alphabetical order)
2. Include patterns, optional keywords for disambiguation
3. Add test fixture in `tests/fixtures/<language>/`
4. Run `cargo test` to validate

## Testing

Tests validate the binary output against fixture files with embedded expected counts:

```
tests/fixtures/
├── rust/
├── python/
├── bash/
└── ... (67+ languages)
```

Each fixture contains special comments with expected line counts that the test harness parses and validates against actual output.

## Dependencies

Key crates:
- `clap` - CLI parsing
- `ignore` - Gitignore-aware directory walking
- `memmap2` - Memory-mapped I/O
- `encoding_rs` - Character encoding detection
- `askama` - Template rendering (HTML/Markdown output)
- `serde`/`serde_json` - Serialization

## Feature Flags

- `default = ["html", "markdown"]`
- `html` - Enable HTML report output
- `markdown` - Enable Markdown report output
