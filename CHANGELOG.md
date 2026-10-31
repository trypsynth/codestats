# Changelog

## Unreleased

- Xcode prefix headers (`.pch`) are counted, as C, C++ or Objective-C like `.h` files
- A file of a format that can be a program, like a BRLTTY contraction table (`.ctb`), is counted as the language of its shebang when it starts with one; the new `shebang_overrides` field in `languages.json5` marks such formats
- Added `cs diff <FROM> [PATH]`, which compares each language's files and code, comment and blank lines between a git revision and the working tree (or another revision with `--to`), in every output format. Revisions are read straight from git's object store, with nothing checked out; on the Linux kernel a diff against `HEAD` takes about 2 seconds
- Added `--git` (`git_tracked_only` in config files) to count only the files git tracks, including in submodules, read from git's index instead of walking the directory; hidden, excluded and generated files are still left out, and `.gitignore` doesn't apply to tracked files
- **Breaking:** JSON output has a new, consistent shape with a `schema_version` (1). Line counts are a `lines` object (`total`, `code`, `comment`, `blank`, `shebang`) at every level, sizes are `bytes`, file counts are `file_count`, and per-file details are `files`. `summary` is now `totals`, `analysis_path` is `path`, a directory's `path` is its `name`, and `languages_hidden`/`dirs_hidden` are `hidden_languages`/`hidden_directories`. Percentages, averages and `*_human` sizes are gone from JSON, as they're derived from the counts; the other formats still show them

## 0.8.4

- Added Texinfo (`.texi`, `.texinfo`, `.txi`), XAML (`.xaml`), and braille tables: liblouis (`.ctb`, `.cti`, `.utb`, `.uti`, `.dis`) and BRLTTY (`.ktb`, `.kti`, `.ttb`, `.tti`, `.atb`, `.ati`)
- 6502 assembly (`.s65`, `.a65`) counts as Assembly
- Dependency files that compilers write for Make (`.d`) are no longer counted as D; D files are recognized by their keywords and doc comments
- `.tmpl` and `.gotxt` files count as Go Template, `.vcproj` and `.vcxproj.filters` as MSBuild, and Autoconf's `configure.ac` and `configure.in` as M4
- C3 test files (`.c3t`) count as C3, and lowercase `sconscript`/`sconstruct` files as Python, as NVDA's build uses
- 20-40% faster than 0.8.3 (37% on the Linux kernel), with identical counts: lines that can't contain a comment or string are counted without running the classifier, detection samples are tokenized 64 bytes at a time, and each file's path is looked up once
- The README benchmarks now cover eight repositories, including nvda, wxWidgets and rust-lang/rust, at commits pinned in `scripts/bench.sh`, which reproduces them; cloc is no longer included

## 0.8.3

- `.h` files can now be C++: a header is counted as C++ when it has C++-only constructs such as `namespace`, `template`, `std::`, `virtual` or access specifiers, and as C otherwise. On wxWidgets, about 1,870 headers counted as C are now C++, while the Linux kernel's 27,000 headers stay C
- Keywords inside comments no longer count when the languages sharing an extension write comments the same way (C, C++ and Objective-C), so a C header discussing "virtual memory" isn't taken for C++; for the same reason, some Objective-C headers whose documentation comments use `@class` are now correctly C++
- C, C++, Objective-C and LPC keywords now match case-sensitively, so `VIRTUAL` or `TYPEDEF` in an assembly or BASIC include file no longer counts as C++
- Faster, most of all on code with ambiguous extensions like `.c` and `.h`: about 40% faster on the Linux kernel and 17-30% faster on react, cpython and kubernetes, making codestats the fastest of the tools in the benchmarks on every repository

## 0.8.2

- `--by-dir` now works in CSV, TSV, Markdown and HTML output, which previously ignored it and showed the language breakdown; with `--verbose` they also list each directory's files
- Markdown tables now render: a blank line between each table's header and its rows made GitHub-flavored Markdown end the table before its first row
- `-q` with Markdown or HTML output no longer ends with "No recognized programming languages found."
- `--by-dir --sort-by files` now sorts directories by file count rather than line count
- `--min-lines` is now applied before `--top-languages`, so `-t 3` shows three entries whenever three pass the threshold, whatever the sort order
- Binary files made of small 16-bit values (lookup tables and the like) are no longer mistaken for UTF-16 text and counted as code
- Piping into a command that stops reading early, like `cs | head`, no longer prints "Error: Broken pipe" or exits with an error
- Invalid option values now name the option, list the accepted values and suggest the closest one (`-o xml` suggests `html`) instead of "one of the values isn't valid for an argument"
- Files of a language left out by `--lang` or `--exclude-lang` are skipped without being read when their name alone identifies the language

## 0.8.1

- JavaScript and TypeScript regex literals are now recognized, so a regex containing a backtick or `/*` (as in jQuery Validation) no longer turns the following lines into code or comments
- Shebangs using `env -S` (`#!/usr/bin/env -S deno run`) are recognized, and shebangs now match whole interpreter names, so `node` no longer claims `nodemon` and `perl6` is Raku rather than Perl
- Added shebangs for deno, tsx, ts-node, bun, Rscript, pwsh, swift, kotlin, osascript, sbcl, dash, make and raku
- Improved detection for extensions shared by several languages: a keyword repeated many times no longer outweighs several distinct ones (C files that mention "mapping" are no longer LPC), and files where the decision is close are checked against more of their content
- `.inc` files are now also considered as C++, PHP, Makefile or NASM assembly, and are only claimed by a language with real evidence, so C++ includes are no longer counted as Pascal or BitBake and unrelated `.inc` files are left out
- Preprocessor directives written with a space in assembly (`# define`, `# if defined(X)`) are counted as code rather than comments
- HTML and Markdown output are always included; the `html`, `markdown` and `all-formats` features have been removed

## 0.8.0

- Comment markers inside string literals are now ignored, so code like `"src/**/*.rs"` or `printf("/*")` no longer turns the following lines into comments. Supported for C, C++, C#, Objective-C, Java, Kotlin, Swift, Go, Rust, JavaScript, TypeScript, Python, Ruby, PHP, Elixir, Lua, SQL, Haskell, Dart, Scala, Groovy, Zig, Nim, CSS, SCSS, Less, JSONC, JSON5 and Tcl
- Python docstrings are counted as comments, while other triple-quoted strings (`x = """...`) are counted as code
- A block comment start inside a line comment (`// see "/healthz/*"`) no longer opens a block comment, and block comments now only close with their own delimiter
- Fixed comment syntax for about 80 languages, including OpenCL `//`, ABAP `"`, uppercase `REM` in Batch, TLA+, Teal, Smarty and template comments in ERB, EJS and JSP, and corrected which languages' block comments nest
- Improved detection for extensions shared by several languages: `.m` and `.h` files are no longer misdetected as Octave or Limbo, SPARK is told apart from Ada and Yacc from Bison, and `.editorconfig` and `.gitconfig` are no longer INI
- Files with nothing distinctive no longer go uncounted when their extension is shared (bare `.h` and `.c` files fall back to C), and the most specific pattern now wins (`pom.xml` is Maven, `*.blade.php` is Blade)
- Exact filenames are now matched case-sensitively, so Bazel's `BUILD` no longer claims `build` scripts
- C++ now owns `.hh` (previously Hack), Perl `.t` test files are recognized, and config files like `.babelrc` and `Pipfile` count as their real formats
- Fortran is split into free-form Fortran (`.f90` and later) and Fortran (Fixed Form) (`.f`, `.for`, `.f77`), where `C`, `c` and `*` comments only count in the first column
- Added Blade, GAP, Gnuplot, Go Template, Godot Resource, HTTP, Pawn, Smali, systemd and Yuck, plus missing extensions and filenames such as `Containerfile`, `Jenkinsfile`, `.emacs` and `meson.options`
- Removed CSV, Jupyter Notebook, MessagePack, max, Piet and Whitespace, whose line counts weren't meaningful; XML Schema files now count as XML
- `-q` now ends with a compact list of languages and their file counts
- Startup is about 7x faster and per-file language detection is faster, making codestats 1.5-2x faster than tokei and scc on medium-sized repositories
- codestats is now a binary-only crate; the `codestats` library target is no longer published

## 0.7.0

- Added `--by-dir` (`-D`) to show a breakdown by directory instead of by language; combining it with `--verbose` also lists individual files under each directory
- Added `--max-depth N` to cap directory traversal depth
- Added `--min-lines N` to hide entries with fewer than N total lines from the breakdown
- Added `--top-languages N` to limit the breakdown to the top N entries; the output notes how many were omitted when entries are hidden by either flag
- Expanded language coverage, now covering 544 total programming languages.
- File paths in verbose and directory output are now shown relative to the analysis root instead of absolute
- Fixed unsupported block comment definitions for MoonScript
- Generated files such as lockfiles and minified assets are now excluded by default; pass `--include-generated` to count them
- Pre-built binaries are now published to GitHub Releases for Linux (x86_64/ARM64), macOS (Intel/Apple Silicon), and Windows (x86_64/ARM64)
- Renamed `--sort-dir` to `--sort-direction`; dropped the `-i` shorthand from `--no-gitignore` and `-S` from `--symlinks`
- Verbosity is now controlled with `-q`/`--quiet` (summary only) and `-v`/`--verbose` (per-file details); the default shows the language breakdown without file details

## 0.6.0

- Added `--fail-on-error` to exit with a non-zero status code when files are skipped
- Added language filtering: include only specific languages or exclude unwanted ones
- Added many new languages, bringing the total to 464
- Added shell completions via the `completions` subcommand
- Added support for glob-based file and directory exclusions
- Brought back `langs` as a subcommand rather than a top-level flag
- Fixed edge cases in whitespace classification
- Fixed shebang handling when whitespace follows the `#!`
- Improved UTF-16 detection for files without a BOM

## 0.5.0

- Added JSON compact and TSV output formats
- Added a TOML config file for persistent settings
- Added many new languages
- Expanded sorting, number formatting, size unit, and precision controls
- Improved CSV and TSV escaping and blank field handling
- Streamlined CLI flags and improved output consistency

## 0.4.0

- Added HTML and Markdown output formats
- Added more language fixtures and detection tests
- Improved documentation and CLI help text
- Removed the git dependency for gitignore handling
- Reworked display formatting to support sorting and custom output styles

## 0.3.1

- Added memory-mapped I/O for large files
- Added more languages and improved language data
- Improved comment parsing
- Sped up core routines and reduced allocations
- Switched pattern matching to globset and simplified language lookups

## 0.3.0

- Added support for extensionless files with shebangs
- Expanded language detection and added more tests
- Improved output formatting and writer-based rendering

## 0.2.0

- Added many new languages.
- Cleaned up output formatting.
- Improved comment detection.
- Streamlined CLI arguments and usage.

## 0.1.0

- Initial release
