# Codestats

A Rust CLI that summarizes codebases at blazing speed. It counts files, lines, and bytes by language; respects .gitignore and .ignore files; can follow symlinks; optionally displays per-file details; and outputs in either human-readable or machine-friendly formats.

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
- List supported languages (400+): `cs -l`

## CLI reference

Usage: `cs [OPTIONS] [PATH]`

### Arguments

- `PATH` The path to analyze (directory is recursive). Defaults to the current directory.

### Options

- `-l, --langs` List all supported languages and exit.
- `-v, --verbose` Show per-file detail instead of just the summary.
- `-i, --no-gitignore` Do not respect `.gitignore` files.
- `-H, --hidden` Search hidden files and directories.
- `-s, --symlinks` Follow symlinks (use carefully to avoid cycles).
- `-n, --number-style <plain|comma|underscore|space>` Number formatting style. Default: `plain`.
- `-u, --size-units <binary|decimal>` Human-readable size units. Default: `binary`.
- `-p, --precision <0-6>` Percentage precision. Default: `1`.
- `-S, --sort-by <lines|code|comments|blanks|files|size|name>` Sort key for languages (and per-file detail when verbose). Default: `lines`.
- `-d, --sort-dir <asc|desc>` Sort direction. Default: `desc`.
- `-o, --output <human|json|csv|markdown|html>` Output format. Default: `human`.
- `-h, --help` Print help.
- `-V, --version` Print version.

## Benchmarks

`hyperfine --warmup 1 "target/release/cs /home/quin" "tokei /home/quin"`

| Command | Mean ± σ | Min … Max |
| --- | --- | --- |
| `cs /home/quin` | 2.606 s ± 0.024 s | 2.587 s … 2.643 s |
| `tokei /home/quin` | 9.396 s ± 0.029 s | 9.364 s … 9.442 s |

Codestats ran about 3.6 times faster than tokei on this machine (a modest Beelinks mini PC) when scanning my large home directory with caches warmed up.

## License

Codestats is licensed under the [Zlib License](LICENSE).
