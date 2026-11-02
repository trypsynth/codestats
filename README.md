# codestats (cs)

[![Crates.io](https://img.shields.io/crates/v/codestats.svg)](https://crates.io/crates/codestats)
[![License](https://img.shields.io/crates/l/codestats.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.88+-blue)](#)

The fastest code counter around. Think that's wrong? Open an issue with [numbers](#benchmarks) and I'll change this line.

codestats counts the code, comment, blank, and shebang lines in a codebase by language, and reports them as readable text, JSON, CSV, TSV, Markdown, or HTML. Built for quick audits, comparisons, scripts, or randomly being curious how much source code is in your home directory.

## Highlights

- Counts files, lines, and bytes by language
- Counts code embedded in other files by its own rules, and shows how much of each language is there: scripts and styles in HTML, Vue, Svelte, Astro and HTML templates, the HTML around PHP's `<?php ... ?>` blocks, and code blocks in Markdown, AsciiDoc, Org and reStructuredText
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

This needs Rust 1.88 or newer. The minimum version only goes up in a minor release, never a patch release.

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

### Count several paths together

```bash
cs src tests build.rs
```

Files are listed by the path they were reached by (`src/main.rs`), and a path given twice or inside another is only counted once.

### Verbose per-file detail for `src/` in JSON

```bash
cs -v src -o json
```

### Compare with an earlier release

```bash
cs diff v1.0.0               # v1.0.0 against the working tree
cs diff v1.0.0 --to v2.0.0   # between two revisions
```

Each language's files, lines and code, comment and blank lines in both versions, with the change. Both sides count the files git tracks, with the same filters, so only real changes show.

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

Every level of a JSON report counts lines the same way. `paths` lists the paths counted together. `files` appears with `--verbose`, and `directories` with `--by-dir`. `cs diff -o json` reports `from` and `to` counts in the same shape.

`schema_version` goes up whenever a field is renamed, removed or changes meaning; new fields can appear without it changing, so read reports in a way that ignores fields you don't know. The same promise covers CSV and TSV columns and the config file's keys: from 1.0 on, those only break in a new major version.

A language or file with code embedded in it, like the Rust in a Markdown file, has an `embedded` list of those languages and their lines, which are also counted in its own `lines`:

```json
"embedded": [{ "name": "Rust", "lines": { "total": 120, "code": 96, "comment": 14, "blank": 10, "shebang": 0 } }]
```

```json
{
	"schema_version": 2,
	"paths": ["src"],
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

Usage: `cs [OPTIONS] [PATH]...` (defaults to the current directory)

- `-q, --quiet` Show totals only, no language breakdown
- `-v, --verbose` Show per-file details in addition to the language breakdown
- `--no-gitignore` Do not respect `.gitignore`
- `--include-generated` Count generated files. By default, codestats does not count them. Generated files are lockfiles, minified assets, and files with a "generated" header. For examples of these headers, see [Generated files](#generated-files)
- `--git` Only count files tracked by git, including in submodules, so untracked scratch files and downloads are left out. `.gitignore` doesn't apply to what git tracks
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
- `-t, --top-languages <N>` Limit the language breakdown to the top N languages (at least 1; `--quiet` shows totals only)
- `-D, --by-dir` Show a breakdown by directory instead of by language
- `-o, --output <human|json|json-compact|csv|tsv|markdown|html>` Output format. Default: `human`
- `--fail-on-error` Exit with a non-zero status code if any files are skipped due to errors
- `-c, --config <PATH>` Use a TOML config file
- `-h, --help` Print help
- `-V, --version` Print version

`cs` exits with status 0 on success, 1 on an error (including a file skipped with `--fail-on-error`), and 2 when its arguments are invalid.

## Subcommands

- `diff <FROM> [PATH]` Compare line counts by language between a git revision and the working tree, or another revision with `--to <REV>`
- `langs` List all supported languages
- `completions <shell>` Generate shell completions (e.g. `bash`, `zsh`, `fish`)

## Configuration

Codestats can read settings from TOML; anything given on the command line wins. A key it doesn't know is an error, so a typo can't be silently ignored. Search order:

1. `--config <path>` (errors if missing)
2. `./.codestats.toml`
3. `./codestats.toml`
4. `~/.config/codestats/config.toml`
5. `~/.codestats.toml`

### Example TOML config

```toml
paths = ["src", "tests"]  # relative to this file

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

## Generated files

By default, codestats does not count generated files. To count them, use `--include-generated`.

Codestats finds generated files in two ways:

- **By name.** Lockfiles (such as `Cargo.lock` and `package-lock.json`) and minified assets (`*.min.js`, `*.min.css`).
- **By header.** Codestats reads the comments at the start of each file. These comments stop at the first line of code. If they say that a tool made the file, codestats does not count the file. Examples:
  - `// Code generated by stringer; DO NOT EDIT.` (Go)
  - `// Generated by the protocol buffer compiler.  DO NOT EDIT!` (protobuf)
  - `// <auto-generated />` (C#)
  - `// @generated by pegen` (and other tools that write `@generated`)
  - `/* automatically generated by rust-bindgen */`
  - `# This makefile was generated by bakefile.` with `# Do not modify` on a near line

A comment that only mentions generated code does not make a file generated. For example, `// Retrieves the auto-generated key` does not.

## Technical Notes

### Memory-Mapped I/O

For performance, Codestats uses memory-mapped I/O for files >=256KB. This provides significant speedups but requires that files remain stable during analysis.

### Thread Safety

Codestats uses parallel processing to maximize performance. Each worker thread maintains its own statistics which are merged at the end, minimizing lock contention. The tool is safe for concurrent execution on different directories, but should not analyze the same directory simultaneously from multiple processes.

## Accuracy

Every change is checked in CI against [LineJudge](https://github.com/loc-conformance/linejudge), a corpus of tricky files (comment markers inside strings, raw strings, heredocs, languages embedded in other languages and more) judged by the rules codestats declares in [`.linejudge`](.linejudge). The cases codestats still gets wrong are recorded, with notes, in [`.linejudge/recorded/codestats.toml`](.linejudge/recorded/codestats.toml); CI fails if any other case changes its answer. To run it locally, install LineJudge and run `linejudge check --counter codestats` after `cargo build --release`.

## Benchmarks

Measured with [hyperfine](https://github.com/sharkdp/hyperfine) on the repositories below at the commits pinned in [`scripts/bench.sh`](scripts/bench.sh), which reproduces these tables: `hyperfine -N --warmup 2 --runs 10` per tool (5 runs for rust and linux), with a warm file cache and every tool on its default settings.

Mean wall time:

| Repository | Files | Lines | codestats | tokei | scc | gocloc |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| [ripgrep](https://github.com/BurntSushi/ripgrep) | 154 | 65K | **6.4 ms** | 10 ms | 12 ms | 40 ms |
| [cli](https://github.com/cli/cli) | 1,125 | 321K | **10 ms** | 23 ms | 19 ms | 113 ms |
| [nvda](https://github.com/nvaccess/nvda) | 1,158 | 1.6M | **15 ms** | 48 ms | 22 ms | 138 ms |
| [wxWidgets](https://github.com/wxWidgets/wxWidgets) | 4,922 | 2.4M | **41 ms** | 149 ms | 108 ms | 457 ms |
| [cpython](https://github.com/python/cpython) | 5,176 | 3.0M | **39 ms** | 139 ms | 85 ms | 673 ms |
| [kubernetes](https://github.com/kubernetes/kubernetes) | 27,174 | 7.3M | **101 ms** | 356 ms | 222 ms | 2.04 s |
| [rust](https://github.com/rust-lang/rust) | 42,485 | 5.3M | **135 ms** | 457 ms | 235 ms | 2.43 s |
| [linux](https://github.com/torvalds/linux) | 93,350 | 44M | **502 ms** | 1.53 s | 984 ms | 11.0 s |

Peak memory:

| Repository | codestats | tokei | scc | gocloc |
| --- | ---: | ---: | ---: | ---: |
| [ripgrep](https://github.com/BurntSushi/ripgrep) | **7.5 MB** | 7.7 MB | 26 MB | 29 MB |
| [cli](https://github.com/cli/cli) | **7.7 MB** | 8.0 MB | 33 MB | 36 MB |
| [nvda](https://github.com/nvaccess/nvda) | **18 MB** | 30 MB | 38 MB | 38 MB |
| [wxWidgets](https://github.com/wxWidgets/wxWidgets) | **17 MB** | 128 MB | 74 MB | 40 MB |
| [cpython](https://github.com/python/cpython) | **12 MB** | 24 MB | 65 MB | 45 MB |
| [kubernetes](https://github.com/kubernetes/kubernetes) | **17 MB** | 66 MB | 99 MB | 52 MB |
| [rust](https://github.com/rust-lang/rust) | **27 MB** | 86 MB | 130 MB | 67 MB |
| [linux](https://github.com/torvalds/linux) | **72 MB** | 234 MB | 373 MB | 110 MB |

Measured with codestats 0.9.1, tokei 15.0.0, scc 4.1.0, gocloc 0.7.0 and hyperfine 2.0.0 on an AMD Ryzen 7 PRO 5850U (8 cores, 16 threads) with 32 GB of RAM. File and line counts are codestats' own; tools differ slightly in what they count (for example, codestats skips generated and minified files by default).

## License

Codestats is licensed under the [MIT License](LICENSE).
