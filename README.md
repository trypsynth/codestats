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
- Supports 550+ languages out of the box
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

### JSON

Every level of a JSON report counts lines the same way. `files` appears with `--verbose`, and `directories` with `--by-dir`; `schema_version` changes whenever a field is renamed, removed or changes meaning.

```json
{
	"schema_version": 1,
	"path": "src",
	"totals": {
		"file_count": 30,
		"lines": { "total": 5956, "code": 5083, "comment": 331, "blank": 542, "shebang": 0 },
		"bytes": 195221
	},
	"languages": [
		{
			"name": "Rust",
			"file_count": 30,
			"lines": { "total": 5956, "code": 5083, "comment": 331, "blank": 542, "shebang": 0 },
			"bytes": 195221,
			"files": [{ "path": "cli.rs", "lines": { "total": 328, "code": 264, "comment": 39, "blank": 25, "shebang": 0 }, "bytes": 12873 }]
		}
	],
	"hidden_languages": 0
}
```

## Common flags

Usage: `cs [OPTIONS] [PATH]` (defaults to the current directory)

- `-q, --quiet` Show totals only, no language breakdown
- `-v, --verbose` Show per-file details in addition to the language breakdown
- `--no-gitignore` Do not respect `.gitignore`
- `--include-generated` Count generated files (lockfiles, minified assets) which are excluded by default
- `--git` Only count files tracked by git, including in submodules, so untracked scratch files and downloads are left out
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
git_tracked_only = false  # true counts only files git tracks
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

Measured with [hyperfine](https://github.com/sharkdp/hyperfine) on the repositories below at the commits pinned in [`scripts/bench.sh`](scripts/bench.sh), which reproduces these tables: `hyperfine -N --warmup 2 --runs 10` per tool (5 runs for rust and linux), with a warm file cache and every tool on its default settings.

Mean wall time:

| Repository | Files | Lines | codestats | tokei | scc | gocloc |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| [ripgrep](https://github.com/BurntSushi/ripgrep) | 154 | 65K | **5.8 ms** | 10 ms | 12 ms | 37 ms |
| [cli](https://github.com/cli/cli) | 1,125 | 321K | **9.3 ms** | 22 ms | 19 ms | 110 ms |
| [nvda](https://github.com/nvaccess/nvda) | 1,158 | 1.6M | **16 ms** | 48 ms | 22 ms | 133 ms |
| [wxWidgets](https://github.com/wxWidgets/wxWidgets) | 4,582 | 2.3M | **41 ms** | 147 ms | 102 ms | 445 ms |
| [cpython](https://github.com/python/cpython) | 5,165 | 3.0M | **35 ms** | 140 ms | 81 ms | 665 ms |
| [kubernetes](https://github.com/kubernetes/kubernetes) | 27,174 | 7.3M | **99 ms** | 360 ms | 220 ms | 2.04 s |
| [rust](https://github.com/rust-lang/rust) | 42,485 | 5.3M | **122 ms** | 452 ms | 228 ms | 2.39 s |
| [linux](https://github.com/torvalds/linux) | 92,866 | 44M | **498 ms** | 1.54 s | 993 ms | 11.2 s |

Peak memory:

| Repository | codestats | tokei | scc | gocloc |
| --- | ---: | ---: | ---: | ---: |
| [ripgrep](https://github.com/BurntSushi/ripgrep) | **6.9 MB** | 7.8 MB | 27 MB | 29 MB |
| [cli](https://github.com/cli/cli) | **7.2 MB** | 8.1 MB | 32 MB | 36 MB |
| [nvda](https://github.com/nvaccess/nvda) | **17 MB** | 32 MB | 38 MB | 38 MB |
| [wxWidgets](https://github.com/wxWidgets/wxWidgets) | **16 MB** | 127 MB | 75 MB | 40 MB |
| [cpython](https://github.com/python/cpython) | **12 MB** | 24 MB | 65 MB | 45 MB |
| [kubernetes](https://github.com/kubernetes/kubernetes) | **17 MB** | 65 MB | 97 MB | 52 MB |
| [rust](https://github.com/rust-lang/rust) | **27 MB** | 88 MB | 128 MB | 65 MB |
| [linux](https://github.com/torvalds/linux) | **78 MB** | 246 MB | 380 MB | 111 MB |

Measured with codestats 0.8.4, tokei 15.0.0, scc 4.1.0, gocloc 0.7.0 and hyperfine 2.0.0 on an AMD Ryzen 7 PRO 5850U (8 cores, 16 threads) with 32 GB of RAM. File and line counts are codestats' own; tools differ slightly in what they count (for example, codestats skips generated and minified files by default).

## License

Codestats is licensed under the [MIT License](LICENSE).
