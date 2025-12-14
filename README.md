# Codestats

A small CLI tool that summarizes codebases at blazing speed. Highlights include:

- Counts files, lines, and bytes by language
- Can respect .gitignore and .ignore files
- Can follow symlinks
- Can analyze hidden files
- Optionally displays per-file details
- Outputs in customizable human-readable or machine-friendly formats.

## Installation

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

- Analyze the current directory and provide a human-readable report: `cs`
- Get a verbose, per-file detail for `src/` in JSON format: `cs -v src -o json`
- List supported languages (440+): `cs -l`

## Configuration

Codestats can read settings from TOML while keeping full CLI compatibility.

Search order:

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

[display]
number_style = "comma"
size_units = "decimal"
precision = 4
sort_by = "files"
sort_direction = "desc"
output = "human"
```

## CLI reference

Usage: `cs [OPTIONS] [PATH]`

### Arguments

- `PATH` The path to analyze. Defaults to the current directory.

### Options

- `-c, --config <PATH>` Path to a TOML config file.
- `-l, --langs` List all supported languages and exit.
- `-v, --verbose` Show per-file detail instead of just the summary.
- `-i, --no-gitignore` Do not respect `.gitignore` files.
- `-H, --hidden` Search hidden files and directories.
- `-S, --symlinks` Follow symlinks (use carefully to avoid cycles).
- `-n, --number-style <plain|comma|underscore|space>` Number formatting style. Default: `plain`.
- `-u, --size-units <binary|decimal>` Human-readable size units. Default: `binary`.
- `-p, --precision <0-6>` Percentage precision. Default: `1`.
- `-s, --sort-by <lines|code|comments|blanks|files|size|name>` Sort key for languages (and per-file detail when verbose). Default: `lines`.
- `-d, --sort-dir <asc|desc>` Sort direction. Default: `desc`.
- `-o, --output <human|json|json-compact|csv|tsv|markdown|html>` Output format. Default: `human`.
- `-h, --help` Print help.
- `-V, --version` Print version.

## Benchmarks

`hyperfine --warmup 1 "cs ~" "tokei ~"`

| Command | Mean ± σ | Min … Max |
| --- | --- | --- |
| `cs ~` | 1.952 s ± 0.034 s | 1.915 s …  1.997 s |
| `tokei ~` | 7.538 s ± 0.045 s | 7.466 s …  7.609 s |

Codestats ran 3.86 ± 0.07 times faster than tokei on a small Beelinks mini PC.

## License

Codestats is licensed under the [MIT License](LICENSE).
