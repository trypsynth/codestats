# codestats (cs)

[![Crates.io](https://img.shields.io/crates/v/codestats.svg)](https://crates.io/crates/codestats)
[![License](https://img.shields.io/crates/l/codestats.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.88+-blue)](#)

A small CLI tool that summarizes codebases at blazing speed. Built for quick audits, comparisons, scripts, or randomly being curious how much source code is in your home directory.

## Highlights

- Counts files, lines, and bytes by language
- Respects `.gitignore` and `.ignore` when you want it to
- Can follow symlinks and scan hidden files
- Filter by language: include only specific languages or exclude unwanted ones
- Exclude files and directories with glob patterns
- Optional per-file detail for drilling into hot spots
- Outputs in human-readable or machine-friendly formats
- Supports 440+ languages out of the box

## Why Codestats

- Fast on laptops and tiny boxes alike
- Works as a drop-in for dashboards or quick CLI reports
- Friendly defaults with easy overrides through CLI or config

## Install

### With cargo

```bash
cargo install codestats
```

### From source

```bash
git clone https://github.com/trypsynth/codestats
cd codestats
cargo install --path .
```

## Quick start

- Human-readable report of the current directory: `cs`
- Verbose per-file detail for `src/` in JSON: `cs -v src -o json`
- List supported languages: `cs -l`
- Ignore `.gitignore` rules: `cs -i`
- Follow symlinks and include hidden files: `cs -SH`
- Analyze only Rust files: `cs -L rust`
- Analyze everything except tests and documentation: `cs -e 'test_*' -e '*.md'`
- Exclude specific languages: `cs --exclude-lang markdown --exclude-lang toml`

## Output formats

- `human` (default) for terminals
- `json` or `json-compact` for scripts
- `csv` or `tsv` for spreadsheets
- `markdown` or `html` for docs and dashboards

## Common flags

Usage: `cs [OPTIONS] [PATH]` (defaults to the current directory)

- `-v, --verbose` Show per-file detail instead of just the summary
- `-i, --no-gitignore` Do not respect `.gitignore`
- `-H, --hidden` Search hidden files and directories
- `-S, --symlinks` Follow symlinks (avoid cycles)
- `-e, --exclude <PATTERN>` Exclude files or directories matching glob patterns (can be specified multiple times)
- `-L, --lang <LANGUAGE>` Only analyze files of the specified language(s) (can be specified multiple times, cannot be used with `--exclude-lang`)
- `--exclude-lang <LANGUAGE>` Exclude files of the specified language(s) (can be specified multiple times, cannot be used with `--lang`)
- `-n, --number-style <plain|comma|underscore|space>` Number formatting style. Default: `plain`
- `-u, --size-units <binary|decimal>` Human-readable size units. Default: `binary`
- `-p, --precision <0-6>` Percentage precision. Default: `1`
- `-s, --sort-by <lines|code|comments|blanks|files|size|name>` Sort key for languages and per-file detail. Default: `lines`
- `-d, --sort-dir <asc|desc>` Sort direction. Default: `desc`
- `-o, --output <human|json|json-compact|csv|tsv|markdown|html>` Output format. Default: `human`
- `--fail-on-error` Exit with a non-zero status code if any files are skipped due to errors
- `-c, --config <PATH>` Use a TOML config file
- `-l, --langs` List all supported languages and exit
- `-h, --help` Print help
- `-V, --version` Print version

## Configuration

Codestats can read settings from TOML while keeping full CLI compatibility. Search order:

1. `--config <path>` (errors if missing)
2. `./.codestats.toml`
3. `./codestats.toml`
4. `~/.config/codestats/config.toml`
5. `~/.codestats.toml`

### Example TOML config

```toml
[analysis]
verbose = true
respect_gitignore = true
include_hidden = true
follow_symlinks = false
exclude_patterns = ["*.tmp", "test_*", "node_modules/*"]
include_languages = ["rust", "python"]  # Only analyze these languages
# exclude_languages = ["markdown", "toml"]  # Or exclude these (cannot use both)
fail_on_error = false

[display]
number_style = "comma"
size_units = "decimal"
precision = 4
sort_by = "files"
sort_direction = "desc"
output = "human"
```

## Technical Notes

### Memory-Mapped I/O

For performance, Codestats uses memory-mapped I/O for files >=256KB. This provides significant speedups but requires that files remain stable during analysis.

IMPORTANT: Files should not be modified by external processes while being analyzed. Concurrent modifications during a scan can cause undefined behavior.

### 32-bit Platform Limitations

On 32-bit platforms, individual files larger than ~4GB cannot be processed due to address space limitations. The tool will skip such files with an error message. This limitation does not affect 64-bit platforms.

### Thread Safety

Codestats uses parallel processing to maximize performance. Each worker thread maintains its own statistics which are merged at the end, minimizing lock contention. The tool is safe for concurrent execution on different directories, but should not analyze the same directory simultaneously from multiple processes.

## Benchmarks

Run: `hyperfine --warmup 1 "cs ~" "tokei ~"`

| Command | Mean ± σ | Min … Max |
| --- | --- | --- |
| `cs ~` | 1.952 s ± 0.034 s | 1.915 s …  1.997 s |
| `tokei ~` | 7.538 s ± 0.045 s | 7.466 s …  7.609 s |

Codestats ran 3.86 ± 0.07 times faster than tokei on a small Beelinks mini PC.

## License

Codestats is licensed under the [MIT License](LICENSE).
