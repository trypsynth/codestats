# codestats (cs)

[![Crates.io](https://img.shields.io/crates/v/codestats.svg)](https://crates.io/crates/codestats)
[![License](https://img.shields.io/crates/l/codestats.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.88+-blue)](#)

The fastest code counter around. Think that's wrong? Open an issue with [numbers](#benchmarks) and I'll change this line.

codestats counts the lines in a codebase by language. It counts code lines, comment lines, blank lines, and shebang lines. It shows the results as text, JSON, CSV, TSV, Markdown, or HTML. Use it for quick audits, comparisons, and scripts.

## Highlights

- Counts files, lines, and bytes by language.
- Counts code that is inside other files by the rules of its own language. Examples:
  - Scripts and styles in HTML, Vue, Svelte, Astro, and HTML templates.
  - The HTML around the `<?php ... ?>` blocks in PHP files.
  - Code blocks in Markdown, AsciiDoc, Org, and reStructuredText.
- Obeys `.gitignore` and `.ignore` files. You can turn this off.
- Can follow symbolic links and read hidden files.
- Can count only the languages that you select, or all languages except the ones that you select.
- Can exclude files and directories that match glob patterns.
- Can show the counts for each file.
- Does not count generated files, such as lockfiles. You can turn this off.
- Obeys the linguist attributes in `.gitattributes` files, as GitHub does.
- Knows more than 550 languages.
- Reads files in all common text encodings.
- Runs very quickly on small and large computers.

## Install

### Pre-built binaries

Download the binary for your platform from [GitHub Releases](https://github.com/trypsynth/codestats/releases/latest):

| Platform | Download |
| --- | --- |
| Linux x86_64 | [cs-x86_64-unknown-linux-musl](https://github.com/trypsynth/codestats/releases/latest/download/cs-x86_64-unknown-linux-musl) |
| Linux ARM64 | [cs-aarch64-unknown-linux-musl](https://github.com/trypsynth/codestats/releases/latest/download/cs-aarch64-unknown-linux-musl) |
| macOS Intel | [cs-x86_64-apple-darwin](https://github.com/trypsynth/codestats/releases/latest/download/cs-x86_64-apple-darwin) |
| macOS Apple Silicon | [cs-aarch64-apple-darwin](https://github.com/trypsynth/codestats/releases/latest/download/cs-aarch64-apple-darwin) |
| Windows x86_64 | [cs-x86_64-pc-windows-gnu.exe](https://github.com/trypsynth/codestats/releases/latest/download/cs-x86_64-pc-windows-gnu.exe) |
| Windows ARM64 | [cs-aarch64-pc-windows-gnullvm.exe](https://github.com/trypsynth/codestats/releases/latest/download/cs-aarch64-pc-windows-gnullvm.exe) |

On Linux and macOS, do these steps after the download:

1. Make the binary executable.
2. Move the binary to a directory on your `PATH`.

```bash
chmod +x cs-*
sudo mv cs-* /usr/local/bin/cs
```

### With cargo

```bash
cargo install codestats
```

You must have Rust 1.88 or newer. We increase the minimum Rust version only in a minor release. We do not increase it in a patch release.

### From source

```bash
git clone https://github.com/trypsynth/codestats
cd codestats
cargo install --path .
```

## Quick start

### Count the current directory

```bash
cs
```

### Count more than one path

```bash
cs src tests build.rs
```

codestats adds the counts of all the paths together. Each file shows with the path that you gave (for example, `src/main.rs`). If you give a path two times, or a path that is in a different path that you gave, codestats counts its files one time only.

### Count the files in a list

```bash
git ls-files -z | cs -
fd -e rs | cs -
```

The path `-` tells codestats to read the paths from standard input. Put one path on each line, or end each path with a NUL character. codestats counts each file in the list one time. It walks each directory in the list.

codestats applies the same filters as in a walk, but it does not apply `.gitignore`. For example, it does not count hidden files unless you use `-H`. A symbolic link to a file counts as the file. codestats follows a symbolic link to a directory only with `--symlinks`.

### Show the counts for each file in `src/`, as JSON

```bash
cs -v src -o json
```

### Compare with an earlier release

```bash
cs diff v1.0.0               # Compare v1.0.0 with the working tree.
cs diff v1.0.0 --to v2.0.0   # Compare two revisions.
```

For each language, the output shows the counts in the two versions and the change. The counts are files, lines, code lines, comment lines, and blank lines. Both versions use the files that git tracks and the same filters. Thus, the output shows only real changes.

### Show only the 10 largest languages

```bash
cs -t 10
```

### Count only Rust files

```bash
cs -L rust
```

### Exclude files that match a glob pattern

```bash
cs -e 'test_*' -e '*.md'
```

### Exclude languages

```bash
cs --exclude-lang markdown --exclude-lang toml
```

### Follow symbolic links and include hidden files

```bash
cs --symlinks -H
```

### Count the files that `.gitignore` excludes

```bash
cs --no-gitignore
```

### Show the list of languages

```bash
cs langs
```

### Show the counts by directory

```bash
cs -D
```

### Make shell completions

```bash
cs completions <shell>
```

## Output formats

- `human` (default): for terminals.
- `json` or `json-compact`: for scripts.
- `csv` or `tsv`: for spreadsheets.
- `markdown` or `html`: for documents and dashboards.

### JSON

All levels of a JSON report count lines in the same way. `paths` is the list of paths that codestats counted. `files` shows only with `--verbose`. `directories` shows only with `--by-dir`. `cs diff -o json` shows its `from` counts and its `to` counts in the same shape.

If code from a different language is inside a language or a file, the language or file has an `embedded` list. An example is Rust code in a Markdown file. The list gives each embedded language and its lines. The `lines` of the language or file also include these lines.

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

From version 1.0, these items change only in a new major version:

- The JSON fields.
- The CSV columns and the TSV columns.
- The keys of the configuration file.

When we rename a JSON field, remove a JSON field, or change what a JSON field means, we increase `schema_version`. We can add new fields and keep the same `schema_version`. Thus, your scripts must ignore the fields that they do not know.

## Common flags

Usage: `cs [OPTIONS] [PATH]...`. If you do not give a path, codestats counts the current directory. If the path is `-`, codestats reads the paths from standard input.

- `-q, --quiet`: Show only the totals. Do not show the languages.
- `-v, --verbose`: Show the counts for each file, and the languages.
- `--no-gitignore`: Ignore the `.gitignore` files.
- `--no-gitattributes`: Ignore the linguist attributes in the `.gitattributes` files. Refer to [Linguist attributes](#linguist-attributes).
- `--include-generated`: Count generated files. Refer to [Generated files](#generated-files).
- `--git`: Count only the files that git tracks, also in submodules. This flag excludes files that git does not track, such as downloads. `.gitignore` does not apply to files that git tracks.
- `--max-depth <N>`: Go down a maximum of N directory levels.
- `--min-lines <N>`: Do not show languages that have fewer than N lines.
- `-H, --hidden`: Include hidden files and directories.
- `--symlinks`: Follow symbolic links. Make sure that no link makes a loop.
- `-e, --exclude <PATTERN>`: Exclude the files and directories that match the glob pattern. You can use this flag more than one time.
- `-L, --lang <LANGUAGE>`: Count only this language. You can use this flag more than one time. You cannot use it with `--exclude-lang`.
- `--exclude-lang <LANGUAGE>`: Do not count this language. You can use this flag more than one time. You cannot use it with `--lang`.
- `-n, --number-style <plain|comma|underscore|space>`: The style of numbers. Default: `plain`.
- `-u, --size-units <binary|decimal>`: The units of sizes. Default: `binary`.
- `-p, --precision <0-6>`: The number of decimal places in percentages. Default: `1`.
- `-s, --sort-by <lines|code|comments|blanks|files|size|name>`: The sort key for languages and files. Default: `lines`.
- `-d, --sort-direction <asc|desc>`: The sort direction. Default: `desc`.
- `-t, --top-languages <N>`: Show only the N largest languages. N must be 1 or more. To show only the totals, use `--quiet`.
- `-D, --by-dir`: Show the counts by directory, not by language.
- `-o, --output <human|json|json-compact|csv|tsv|markdown|html>`: The output format. Default: `human`.
- `--fail-on-error`: If an error stops codestats from reading a file, exit with an error status.
- `-c, --config <PATH>`: Use this TOML configuration file.
- `-h, --help`: Show the help.
- `-V, --version`: Show the version.

### Exit status

- `0`: The count is complete.
- `1`: An error occurred. With `--fail-on-error`, a file that codestats could not read is an error.
- `2`: The arguments are not correct.

## Subcommands

- `diff <FROM> [PATH]`: Compare the counts by language between a git revision and the working tree. To compare with a different revision, use `--to <REV>`.
- `langs`: Show all the languages that codestats knows.
- `completions <shell>`: Make shell completions, for example for `bash`, `zsh`, or `fish`.
- `init`: Make a configuration file that shows all the settings.

## Configuration

codestats can read its settings from a TOML file. A flag on the command line has priority over the same setting in the file. If the file has a key that codestats does not know, codestats stops with an error. Thus, a spelling mistake cannot go unseen.

codestats uses the first file that it finds in this list:

1. The file that you give with `--config <path>`. If this file is missing, codestats stops with an error.
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
git_tracked_only = false  # If true, count only the files that git tracks.
include_hidden = true
follow_symlinks = false
exclude_patterns = ["languages.json5", "*.tmp", "node_modules/*"]  # File names or glob patterns.
include_languages = ["rust", "python"]  # Count only these languages.
# exclude_languages = ["markdown", "toml"]  # Or do not count these languages. You cannot use both.
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

codestats does not count generated files. To count them, use `--include-generated`.

codestats finds generated files in three ways:

- **By attribute.** The `linguist-generated` attribute in a `.gitattributes` file. Refer to [Linguist attributes](#linguist-attributes).
- **By name.** Lockfiles (for example, `Cargo.lock` and `package-lock.json`) and minified files (`*.min.js` and `*.min.css`).
- **By header.** codestats reads the comments at the start of each file. These comments stop at the first line of code. If the comments say that a tool made the file, codestats does not count the file. Examples:
  - `// Code generated by stringer; DO NOT EDIT.` (Go)
  - `// Generated by the protocol buffer compiler.  DO NOT EDIT!` (protobuf)
  - `// <auto-generated />` (C#)
  - `// @generated by pegen` (and other tools that write `@generated`)
  - `/* automatically generated by rust-bindgen */`
  - `# This makefile was generated by bakefile.`, with `# Do not modify` on a near line

A comment that only mentions generated code does not make a file generated. For example, `// Retrieves the auto-generated key` does not.

## Linguist attributes

GitHub uses the linguist attributes in `.gitattributes` files for its language statistics. codestats uses them in the same way:

| Attribute | Result |
| --- | --- |
| `linguist-generated` | The file is generated. codestats does not count it. To count it, use `--include-generated`. |
| `-linguist-generated` or `linguist-generated=false` | The file is not generated. codestats counts it, also if its name or header looks generated. |
| `linguist-vendored` | codestats does not count the file. |
| `linguist-documentation` | codestats does not count the file. |
| `linguist-language=Name` | codestats counts the file as the language `Name`. Refer to the rules below this table. |

codestats ignores `linguist-language` in these conditions:

- codestats does not know the language `Name`.
- The name of the file gives exactly one language that codestats knows. For example, codestats counts `*.bgt` files as BGT and `*.nvgt` files as NVGT. Many projects map these files to AngelScript because GitHub does not know BGT or NVGT.

If the name of the file gives no language or more than one language (for example, `*.h` or `*.pp`), codestats uses `Name`.

Example `.gitattributes` file:

```gitattributes
**/zz_generated.*.go linguist-generated
lib/libc/**          linguist-vendored
*.fixed              linguist-language=Rust
```

codestats reads the `.gitattributes` files in the directories that it counts. It also reads the `.gitattributes` files in the directories above, up to the root of the repository. A file in a lower directory has priority over a file in a higher directory. In one file, a lower line has priority over a higher line. codestats also expands macros, such as `[attr]generated linguist-generated`.

With `--git` and with `cs diff`, codestats uses the `.gitattributes` files in the working tree for all revisions. Thus, both sides of a comparison use the same rules.

To ignore the linguist attributes, use `--no-gitattributes`.

## Technical notes

### Memory-mapped files

codestats maps files of 256 KB or more into memory. This makes codestats faster. Do not change files while codestats counts them.

### Threads

codestats counts files on many threads at the same time. Each thread keeps its own counts. At the end, codestats adds the counts together. You can run codestats on different directories at the same time. Do not count the same directory with more than one codestats process at the same time.

## Accuracy

Each change to codestats must pass the [LineJudge](https://github.com/loc-conformance/linejudge) tests in CI. LineJudge is a set of difficult files. Examples are comment markers in strings, raw strings, heredocs, and languages in other languages. LineJudge compares the counts of codestats with the rules in [`.linejudge`](.linejudge).

[`.linejudge/recorded/codestats.toml`](.linejudge/recorded/codestats.toml) gives the cases that codestats counts incorrectly, with notes. If the count of a different case changes, CI fails.

To run LineJudge on your computer:

1. Install LineJudge.
2. Run `cargo build --release`.
3. Run `linejudge check --counter codestats`.

## Benchmarks

We measured these times with [hyperfine](https://github.com/sharkdp/hyperfine). [`scripts/bench.sh`](scripts/bench.sh) gives the commit of each repository, and it makes these tables again. Each tool ran with `hyperfine -N --warmup 2 --runs 10` (5 runs for rust and linux), with its default settings. The files were in the file cache.

Mean time:

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

Versions: codestats 0.9.1, tokei 15.0.0, scc 4.1.0, gocloc 0.7.0, and hyperfine 2.0.0. Computer: AMD Ryzen 7 PRO 5850U (8 cores, 16 threads) with 32 GB of RAM. The counts of files and lines are from codestats. The tools do not count all the same files. For example, codestats does not count generated files.

## License

codestats has the [MIT License](LICENSE).
