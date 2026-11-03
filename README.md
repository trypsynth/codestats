# codestats (cs)

[![Crates.io](https://img.shields.io/crates/v/codestats.svg)](https://crates.io/crates/codestats)
[![License](https://img.shields.io/crates/l/codestats.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.88+-blue)](#)

The fastest code counter around. Think that's wrong? Open an issue with [numbers](#benchmarks) and I'll change this line.

codestats counts the code, comment, blank, and shebang lines in a codebase by language, and reports them as text, JSON, CSV, TSV, Markdown, or HTML. Use it for audits, comparisons, scripts, or to see how much code is in a directory.

## Features

- Counts files, lines, and bytes by language.
- Counts code embedded in other files by that code's own language rules, including:
  - Scripts and styles in HTML, Vue, Svelte, Astro, and HTML templates.
  - The HTML around `<?php ... ?>` blocks in PHP files.
  - Code blocks in Markdown, AsciiDoc, Org, and reStructuredText.
- Respects `.gitignore` and `.ignore` files.
- Skips generated files, such as lockfiles and files marked `DO NOT EDIT`.
- Reads the linguist attributes in `.gitattributes` files, the same way GitHub does.
- Filters by language, glob pattern, and directory depth.
- Shows counts per file or per directory.
- Recognizes more than 560 languages.
- Handles UTF-8, UTF-16, and legacy text encodings.

## Install

### Download a binary

Download the binary for your platform from [GitHub Releases](https://github.com/trypsynth/codestats/releases/latest):

| Platform | Download |
| --- | --- |
| Linux x86_64 | [cs-x86_64-unknown-linux-gnu](https://github.com/trypsynth/codestats/releases/latest/download/cs-x86_64-unknown-linux-gnu) |
| Linux ARM64 | [cs-aarch64-unknown-linux-gnu](https://github.com/trypsynth/codestats/releases/latest/download/cs-aarch64-unknown-linux-gnu) |
| Linux x86_64, static (musl) | [cs-x86_64-unknown-linux-musl](https://github.com/trypsynth/codestats/releases/latest/download/cs-x86_64-unknown-linux-musl) |
| Linux ARM64, static (musl) | [cs-aarch64-unknown-linux-musl](https://github.com/trypsynth/codestats/releases/latest/download/cs-aarch64-unknown-linux-musl) |
| macOS Intel | [cs-x86_64-apple-darwin](https://github.com/trypsynth/codestats/releases/latest/download/cs-x86_64-apple-darwin) |
| macOS Apple Silicon | [cs-aarch64-apple-darwin](https://github.com/trypsynth/codestats/releases/latest/download/cs-aarch64-apple-darwin) |
| Windows x86_64 | [cs-x86_64-pc-windows-gnu.exe](https://github.com/trypsynth/codestats/releases/latest/download/cs-x86_64-pc-windows-gnu.exe) |
| Windows ARM64 | [cs-aarch64-pc-windows-gnullvm.exe](https://github.com/trypsynth/codestats/releases/latest/download/cs-aarch64-pc-windows-gnullvm.exe) |

The standard Linux binaries need glibc 2.17 or later and run faster than the static ones. Use the static binaries on systems without glibc, such as Alpine.

On Linux and macOS, make the binary executable and move it to a directory on your `PATH`:

```bash
chmod +x cs-*
sudo mv cs-* /usr/local/bin/cs
```

### Install with Cargo

```bash
cargo install codestats
```

This requires Rust 1.88 or later. Release binaries are built with profile-guided optimization, so they run faster than a build from `cargo install`. The minimum Rust version only increases in minor releases, never in patch releases.

### Build from source

```bash
git clone https://github.com/trypsynth/codestats
cd codestats
cargo install --path .
```

## Quick start

Count the current directory:

```bash
cs
```

Count several paths together:

```bash
cs src tests build.rs
```

Files are listed by the path you gave, such as `src/main.rs`. If you give the same path twice, or a path inside another one you gave, its files are counted once.

Count a list of files from standard input:

```bash
git ls-files -z | cs -
fd -e rs | cs -
```

The list can have one path per line or NUL-separated paths. Directories in the list are walked. The usual filters apply, except `.gitignore`, so hidden files are skipped unless you pass `-H`. A symbolic link to a file counts as that file. A symbolic link to a directory is followed only with `--symlinks`.

Show per-file counts for `src/` as JSON:

```bash
cs -v src -o json
```

Compare with an earlier release:

```bash
cs diff v1.0.0               # v1.0.0 against the working tree
cs diff v1.0.0 --to v2.0.0   # two revisions
```

For each language, `cs diff` shows the files, lines, code, comments, and blanks in both versions, and the change. Both sides count the files git tracks, with the same filters, so only real changes appear.

More examples:

```bash
cs -t 10                                         # only the 10 largest languages
cs -L rust                                       # only Rust
cs -e 'test_*' -e '*.md'                         # exclude glob patterns
cs --exclude-lang markdown --exclude-lang toml   # exclude languages
cs --symlinks -H                                 # follow symbolic links, include hidden files
cs --no-gitignore                                # include files .gitignore excludes
cs -D                                            # counts by directory
cs langs                                         # list supported languages
cs completions bash                              # shell completions
```

## Output formats

- `human` (default) for terminals.
- `json` or `json-compact` for scripts.
- `csv` or `tsv` for spreadsheets.
- `markdown` or `html` for documents and dashboards.

### JSON

Every level of a JSON report counts lines the same way. `paths` lists the paths that were counted. `files` appears with `--verbose`, and `directories` with `--by-dir`. `cs diff -o json` reports its `from` and `to` counts in the same shape.

A language or file that contains code in another language, such as Rust in a Markdown file, has an `embedded` list with each embedded language and its lines. Those lines are also included in the language's or file's own `lines`.

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

### Stability

Starting with 1.0, JSON fields, CSV and TSV columns, and configuration file keys only change in a new major version.

`schema_version` increases whenever a JSON field is renamed, removed, or changes meaning. New fields can appear without a version change, so your scripts should ignore fields they don't recognize.

## Command-line options

Usage: `cs [OPTIONS] [PATH]...`

If you don't give a path, codestats counts the current directory. If the path is `-`, it reads the paths to count from standard input.

- `-q, --quiet`: show only the totals.
- `-v, --verbose`: also show the counts for each file.
- `--no-gitignore`: ignore `.gitignore` files.
- `--no-gitattributes`: ignore the linguist attributes in `.gitattributes` files. See [Linguist attributes](#linguist-attributes).
- `--include-generated`: count generated files. See [Generated files](#generated-files).
- `--git`: count only the files that git tracks, including in submodules. `.gitignore` doesn't apply to tracked files.
- `--max-depth <N>`: descend at most N directory levels.
- `--min-lines <N>`: hide languages with fewer than N lines.
- `-H, --hidden`: include hidden files and directories. The `.git` directory is always skipped.
- `--symlinks`: follow symbolic links. Links that form a loop make the walk run forever.
- `-e, --exclude <PATTERN>`: exclude files and directories that match a glob pattern. Repeatable.
- `-L, --lang <LANGUAGE>`: count only this language. Repeatable, or comma-separated. Can't be combined with `--exclude-lang`.
- `--exclude-lang <LANGUAGE>`: don't count this language. Repeatable, or comma-separated. Can't be combined with `--lang`.
- `--ext <EXT>`: count only files with this extension, such as `rs` or `py`. Repeatable, or comma-separated.
- `-n, --number-style <plain|comma|underscore|space>`: how to format numbers. Default: `plain`.
- `-u, --size-units <binary|decimal>`: the units for sizes. Default: `binary`.
- `-p, --precision <0-6>`: decimal places in percentages. Default: `1`.
- `-s, --sort-by <lines|code|comments|blanks|files|size|name>`: how to sort languages and files. Default: `lines`.
- `-d, --sort-direction <asc|desc>`: the sort direction. Default: `desc`.
- `-t, --top-languages <N>`: show only the N largest languages. N must be at least 1; to show only the totals, use `--quiet`.
- `-D, --by-dir`: show counts by directory instead of by language.
- `-o, --output <human|json|json-compact|csv|tsv|markdown|html>`: the output format. Default: `human`.
- `--fail-on-error`: exit with an error if any file couldn't be read.
- `-c, --config <PATH>`: use this configuration file.
- `-h, --help`: show help.
- `-V, --version`: show the version.

### Exit status

- `0`: success.
- `1`: an error, including a file that couldn't be read when you pass `--fail-on-error`.
- `2`: invalid arguments.

## Subcommands

- `diff <FROM> [PATH]`: compare counts by language between a git revision and the working tree, or another revision with `--to <REV>`.
- `langs`: list the supported languages.
- `completions <shell>`: generate shell completions, for example for `bash`, `zsh`, or `fish`.
- `init`: write a configuration file that lists every setting.

## Configuration

codestats can read settings from a TOML file. Options on the command line override the file. An unknown key in the file is an error, so typos don't go unnoticed.

codestats uses the first of these files that exists:

1. The file you pass with `--config <path>`. It's an error if this file doesn't exist.
2. `./.codestats.toml`
3. `./codestats.toml`
4. `~/.config/codestats/config.toml`
5. `~/.codestats.toml`

### Example configuration

```toml
paths = ["src", "tests"]  # Relative to this file.

[analysis]
verbosity = "verbose"
respect_gitignore = true
respect_gitattributes = true  # Use the linguist attributes in .gitattributes files.
git_tracked_only = false  # true counts only the files git tracks.
include_hidden = true
follow_symlinks = false
exclude_patterns = ["languages.json5", "*.tmp", "node_modules/*"]  # File names or glob patterns.
include_languages = ["rust", "python"]  # Count only these languages.
# exclude_languages = ["markdown", "toml"]  # Or skip these. You can't set both.
include_extensions = ["rs", "py"]  # Count only these extensions.
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

By default, codestats skips generated files. To count them, pass `--include-generated`.

A file counts as generated in any of these cases:

- **Its linguist attribute says so.** See [Linguist attributes](#linguist-attributes).
- **Its name says so.** Lockfiles, such as `Cargo.lock` and `package-lock.json`, and minified files, such as `*.min.js` and `*.min.css`.
- **Its header says so.** codestats reads the comments at the start of each file, up to the first line of code, and looks for a note that a tool wrote the file. For example:
  - `// Code generated by stringer; DO NOT EDIT.` (Go)
  - `// Generated by the protocol buffer compiler.  DO NOT EDIT!` (protobuf)
  - `// <auto-generated />` (C#)
  - `// @generated by pegen`, or any other `@generated`
  - `/* automatically generated by rust-bindgen */`
  - `# This makefile was generated by bakefile.` followed closely by `# Do not modify`

A comment that only mentions generated code, such as `// Retrieves the auto-generated key`, doesn't make a file generated.

## Linguist attributes

GitHub uses the linguist attributes in `.gitattributes` files for its language statistics, and codestats uses them the same way:

| Attribute | Effect |
| --- | --- |
| `linguist-generated` | The file is generated, so it's skipped unless you pass `--include-generated`. |
| `-linguist-generated` or `linguist-generated=false` | The file isn't generated, even if its name or header looks like it is. |
| `linguist-vendored` | The file is skipped. |
| `linguist-documentation` | The file is skipped. |
| `linguist-language=Name` | The file is counted as `Name`, with the exceptions below. |

codestats ignores `linguist-language` when:

- It doesn't recognize `Name`.
- The file's name already identifies exactly one language. For example, `*.bgt` files stay BGT and `*.nvgt` files stay NVGT, even though many projects map them to AngelScript because GitHub doesn't recognize BGT or NVGT.

When the file's name matches no language or several, such as `*.h` or `*.pp`, `Name` decides.

For example:

```gitattributes
**/zz_generated.*.go linguist-generated
lib/libc/**          linguist-vendored
*.fixed              linguist-language=Rust
```

codestats reads the `.gitattributes` files in the directories it counts, and in the directories above them up to the repository root. A file in a deeper directory overrides one higher up, and within a file, later lines override earlier ones. Macros, such as `[attr]generated linguist-generated`, are expanded.

With `--git` and `cs diff`, the working tree's `.gitattributes` files apply to every revision, so both sides of a comparison follow the same rules.

To ignore linguist attributes, pass `--no-gitattributes`.

## Technical notes

### Memory-mapped files

codestats memory-maps files of 64 KB or larger, which makes reading them faster. Don't modify files while codestats is counting them.

### Threads

codestats counts files on several threads, each keeping its own totals, and combines them at the end. You can run several codestats processes on different directories at once, but don't run more than one on the same directory.

## Accuracy

Every change is checked in CI against [LineJudge](https://github.com/loc-conformance/linejudge), a collection of tricky files: comment markers in strings, raw strings, heredocs, languages embedded in other languages, and more. LineJudge checks codestats against the counting rules declared in [`.linejudge`](.linejudge).

The cases codestats still gets wrong are recorded, with notes, in [`.linejudge/recorded/codestats.toml`](.linejudge/recorded/codestats.toml). CI fails if any other case changes.

To run LineJudge locally:

1. Install LineJudge.
2. Run `cargo build --release`.
3. Run `linejudge check --counter codestats`.

## Benchmarks

All times are for files already in the file cache, on an AMD Ryzen 7 PRO 5850U (8 cores, 16 threads) with 32 GB of RAM. They use the x86-64 Linux glibc binary that CI built at commit `8ebd313`, which is the same build a release ships.

### Equal work

In these tables, every tool counts the same files. [linebench](https://github.com/loc-conformance/linebench) pins each repository's commit, turns off each tool's filters, and checks that each tool's file and line counts are within 1% of the repository's.

Mean time:

| Repository | Files | Lines | codestats | mezura | scc | tokei |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| [cpython](https://github.com/python/cpython) | 3,554 | 2.2M | **21 ms** | 24 ms | 53 ms | 110 ms |
| [jdk](https://github.com/openjdk/jdk) | 61,420 | 13M | **131 ms** | 157 ms | 350 ms | 901 ms |
| [linux](https://github.com/torvalds/linux) | 63,738 | 36M | **232 ms** | 252 ms | 658 ms | 1.17 s |

Peak memory:

| Repository | codestats | mezura | scc | tokei |
| --- | ---: | ---: | ---: | ---: |
| [cpython](https://github.com/python/cpython) | **11 MB** | 18 MB | 46 MB | 18 MB |
| [jdk](https://github.com/openjdk/jdk) | **18 MB** | 28 MB | 94 MB | 57 MB |
| [linux](https://github.com/torvalds/linux) | **78 MB** | 124 MB | 349 MB | 249 MB |

To reproduce these tables, run the following command with linebench 0.3.1, after running `linebench fetch --counters all --corpus all`:

```sh
linebench run cpython,linux,jdk --counters scc,tokei,mezura,codestats@dev --given codestats@dev=path/to/cs --definition codestats@dev=.linebench/codestats.toml --add .linebench
```

Each tool ran 30 times, half of them in reverse order. These tables measure codestats, mezura 3.3.0, scc 4.1.0, and tokei 15.0.0.

### Default settings

In these tables, every tool runs with no flags, which is how most people use it. The tools don't count the same files, so compare these times with care. For example, mezura counts 253K lines in nvda, where codestats counts 1.6M, because it leaves out Gettext translation files. The file and line counts in the table are from codestats.

Mean time:

| Repository | Files | Lines | codestats | mezura | tokei | scc | gocloc |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [ripgrep](https://github.com/BurntSushi/ripgrep) | 154 | 65K | **4.1 ms** | 6.9 ms | 9.3 ms | 12 ms | 41 ms |
| [cli](https://github.com/cli/cli) | 1,102 | 312K | **6.5 ms** | 10 ms | 20 ms | 20 ms | 117 ms |
| [nvda](https://github.com/nvaccess/nvda) | 1,158 | 1.6M | 13 ms | **9.8 ms** | 44 ms | 21 ms | 144 ms |
| [wxWidgets](https://github.com/wxWidgets/wxWidgets) | 4,491 | 2.3M | 29 ms | **25 ms** | 82 ms | 108 ms | 475 ms |
| [cpython](https://github.com/python/cpython) | 4,947 | 2.5M | **26 ms** | 31 ms | 144 ms | 87 ms | 719 ms |
| [kubernetes](https://github.com/kubernetes/kubernetes) | 22,693 | 5.2M | **71 ms** | 74 ms | 300 ms | 242 ms | 2.17 s |
| [rust](https://github.com/rust-lang/rust) | 44,334 | 5.1M | 104 ms | **102 ms** | 409 ms | 261 ms | 2.57 s |
| [linux](https://github.com/torvalds/linux) | 92,811 | 44M | **319 ms** | 342 ms | 1.45 s | 1.05 s | 11.4 s |

Peak memory:

| Repository | codestats | mezura | tokei | scc | gocloc |
| --- | ---: | ---: | ---: | ---: | ---: |
| [ripgrep](https://github.com/BurntSushi/ripgrep) | **7.3 MB** | 8.1 MB | 7.8 MB | 26 MB | 30 MB |
| [cli](https://github.com/cli/cli) | **8.2 MB** | 9.3 MB | 8.2 MB | 32 MB | 36 MB |
| [nvda](https://github.com/nvaccess/nvda) | 19 MB | **9.5 MB** | 27 MB | 38 MB | 38 MB |
| [wxWidgets](https://github.com/wxWidgets/wxWidgets) | **19 MB** | 20 MB | 23 MB | 73 MB | 40 MB |
| [cpython](https://github.com/python/cpython) | **14 MB** | 22 MB | 25 MB | 65 MB | 46 MB |
| [kubernetes](https://github.com/kubernetes/kubernetes) | **18 MB** | 23 MB | 63 MB | 97 MB | 52 MB |
| [rust](https://github.com/rust-lang/rust) | **31 MB** | 43 MB | 98 MB | 127 MB | 67 MB |
| [linux](https://github.com/torvalds/linux) | **95 MB** | 158 MB | 278 MB | 387 MB | 111 MB |

To reproduce these tables, run `CS=path/to/cs scripts/bench.sh`, which pins each repository's commit. It uses hyperfine 2.0.0 and these tools: mezura 3.3.0, tokei 15.0.0, scc 4.1.0, and gocloc 0.7.0.

## License

codestats is licensed under the [MIT License](LICENSE).
