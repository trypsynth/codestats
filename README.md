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
- Supports 540+ languages out of the box
- Fully encoding-aware
- Runs insanely quickly on anything from a tiny laptop to a large desktop

## Install

### Pre-built binaries

Download the latest binary for your platform from [GitHub Releases](https://github.com/trypsynth/codestats/releases/latest):

| Platform | Download |
| --- | --- |
| Linux x86_64 | [cs-x86_64-unknown-linux-musl](https://github.com/trypsynth/codestats/releases/latest/download/cs-x86_64-unknown-linux-musl) |
| Linux ARM64 | [cs-aarch64-unknown-linux-musl](https://github.com/trypsynth/codestats/releases/latest/download/cs-aarch64-unknown-linux-musl) |
| macOS Intel | [cs-x86_64-apple-darwin](https://github.com/trypsynth/codestats/releases/latest/download/cs-x86_64-apple-darwin) |
| macOS Apple Silicon | [cs-aarch64-apple-darwin](https://github.com/trypsynth/codestats/releases/latest/download/cs-aarch64-apple-darwin) |
| Windows x86_64 | [cs-x86_64-pc-windows-gnu.exe](https://github.com/trypsynth/codestats/releases/latest/download/cs-x86_64-pc-windows-gnu.exe) |
| Windows ARM64 | [cs-aarch64-pc-windows-gnullvm.exe](https://github.com/trypsynth/codestats/releases/latest/download/cs-aarch64-pc-windows-gnullvm.exe) |

On Linux and macOS, make the binary executable and move it somewhere on your `PATH`:

```bash
chmod +x cs-*
sudo mv cs-* /usr/local/bin/cs
```

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

### Analyze the current directory

```bash
cs
```

### Verbose per-file detail for `src/` in JSON

```bash
cs -v src -o json
```

### Show only the top 10 languages

```bash
cs -t 10
```

### Analyze only Rust files

```bash
cs -L rust
```

### Exclude files by glob pattern

```bash
cs -e 'test_*' -e '*.md'
```

### Exclude specific languages

```bash
cs --exclude-lang markdown --exclude-lang toml
```

### Follow symlinks and include hidden files

```bash
cs --symlinks -H
```

### Ignore `.gitignore` rules

```bash
cs -i
```

### List supported languages

```bash
cs langs
```

### Show breakdown by directory

```bash
cs -D
```

### Generate shell completions

```bash
cs completions <shell>
```

## Output formats

- `human` (default) for terminals
- `json` or `json-compact` for scripts
- `csv` or `tsv` for spreadsheets
- `markdown` or `html` for docs and dashboards

## Common flags

Usage: `cs [OPTIONS] [PATH]` (defaults to the current directory)

- `-q, --quiet` Show totals only, no language breakdown
- `-v, --verbose` Show per-file details in addition to the language breakdown
- `--no-gitignore` Do not respect `.gitignore`
- `--include-generated` Count generated files (lockfiles, minified assets) which are excluded by default
- `--max-depth <N>` Limit directory traversal to N levels deep
- `--min-lines <N>` Hide languages with fewer than N total lines
- `-H, --hidden` Search hidden files and directories
- `--symlinks` Follow symlinks (avoid cycles)
- `-e, --exclude <PATTERN>` Exclude files or directories matching glob patterns (can be specified multiple times)
- `-L, --lang <LANGUAGE>` Only analyze files of the specified language(s) (can be specified multiple times, cannot be used with `--exclude-lang`)
- `--exclude-lang <LANGUAGE>` Exclude files of the specified language(s) (can be specified multiple times, cannot be used with `--lang`)
- `-n, --number-style <plain|comma|underscore|space>` Number formatting style. Default: `plain`
- `-u, --size-units <binary|decimal>` Human-readable size units. Default: `binary`
- `-p, --precision <0-6>` Percentage precision. Default: `1`
- `-s, --sort-by <lines|code|comments|blanks|files|size|name>` Sort key for languages and per-file detail. Default: `lines`
- `-d, --sort-direction <asc|desc>` Sort direction. Default: `desc`
- `-t, --top-languages <N>` Limit the language breakdown to the top N languages
- `-D, --by-dir` Show a breakdown by directory instead of by language
- `-o, --output <human|json|json-compact|csv|tsv|markdown|html>` Output format. Default: `human`
- `--fail-on-error` Exit with a non-zero status code if any files are skipped due to errors
- `-c, --config <PATH>` Use a TOML config file
- `-h, --help` Print help
- `-V, --version` Print version

## Subcommands

- `langs` List all supported languages
- `completions <shell>` Generate shell completions (e.g. `bash`, `zsh`, `fish`)

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
verbosity = "verbose"
respect_gitignore = true
include_hidden = true
follow_symlinks = false
exclude_patterns = ["languages.json5", "*.tmp", "node_modules/*"]  # filenames or globs
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
top_languages = 10
by_dir = false
```

## Technical Notes

### Memory-Mapped I/O

For performance, Codestats uses memory-mapped I/O for files >=256KB. This provides significant speedups but requires that files remain stable during analysis.

### Thread Safety

Codestats uses parallel processing to maximize performance. Each worker thread maintains its own statistics which are merged at the end, minimizing lock contention. The tool is safe for concurrent execution on different directories, but should not analyze the same directory simultaneously from multiple processes.

## Benchmarks

Measured with [hyperfine](https://github.com/sharkdp/hyperfine) on shallow clones of each repository with a warm file cache, every tool running with its default settings: `hyperfine -N --warmup 2 --runs 10 '<tool> <repo>'` (5 runs for Linux; cloc, being much slower, got 1 warmup and 5 runs, or 3 for Linux).

Mean wall time:

| Repository | Files | Lines | codestats | tokei | scc | gocloc | cloc |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [ripgrep](https://github.com/BurntSushi/ripgrep) | 154 | 65K | **5.8 ms** | 9.2 ms | 12.1 ms | 38.6 ms | 1.43 s |
| [react](https://github.com/facebook/react) | 6,963 | 1.0M | **35 ms** | 65 ms | 50 ms | 376 ms | 4.17 s |
| [cpython](https://github.com/python/cpython) | 5,092 | 3.0M | **68 ms** | 141 ms | 79 ms | 667 ms | 11.7 s |
| [kubernetes](https://github.com/kubernetes/kubernetes) | 27,124 | 7.2M | **177 ms** | 356 ms | 213 ms | 1.99 s | 68.5 s |
| [linux](https://github.com/torvalds/linux) | 92,954 | 44M | 1.36 s | 1.55 s | **994 ms** | 10.8 s | 130 s |

Peak memory:

| Repository | codestats | tokei | scc | gocloc | cloc |
| --- | ---: | ---: | ---: | ---: | ---: |
| ripgrep | **6.4 MB** | 7.5 MB | 26 MB | 28 MB | 28 MB |
| react | **8.7 MB** | 20 MB | 47 MB | 40 MB | 40 MB |
| cpython | **10 MB** | 24 MB | 61 MB | 43 MB | 72 MB |
| kubernetes | **14 MB** | 52 MB | 99 MB | 49 MB | 115 MB |
| linux | **58 MB** | 225 MB | 366 MB | 104 MB | 338 MB |

Measured with codestats 0.8.1, tokei 15.0.0, scc 4.1.0, gocloc 0.7.0, cloc 2.10 and hyperfine 2.0.0 on an AMD Ryzen 7 PRO 5850U (8 cores, 16 threads) with 32 GB of RAM. File and line counts are codestats' own; tools differ slightly in what they count (for example, codestats skips generated and minified files by default).

## License

Codestats is licensed under the [MIT License](LICENSE).
