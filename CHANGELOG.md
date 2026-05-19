# Changelog

## 0.7.0

- Added `--by-dir` (`-D`) to show a breakdown by directory instead of by language; combining it with `--verbose` also lists individual files under each directory
- Added `--max-depth N` to cap directory traversal depth
- Added `--min-lines N` to hide entries with fewer than N total lines from the breakdown
- Added `--top-languages N` to limit the breakdown to the top N entries; the output notes how many were omitted when entries are hidden by either flag
- File paths in verbose and directory output are now shown relative to the analysis root instead of absolute
- Fixed unsupported block comment definitions for MoonScript
- Generated files such as lockfiles and minified assets are now excluded by default; pass `--include-generated` to count them
- Pre-built binaries are now published to GitHub Releases for Linux (x86_64/ARM64), macOS (Intel/Apple Silicon), and Windows (x86_64/ARM64)
- Renamed `--sort-dir` to `--sort-direction`; dropped the `-i` shorthand from `--no-gitignore` and `-S` from `--symlinks`
- Verbosity is now controlled with `-q`/`--quiet` (summary only) and `-v`/`--verbose` (per-file details); the default shows the language breakdown without file details

## 0.6.0

- Added the ability to filter in or out certain languages from the analysis
- Added support for glob-based path exclusions
- Added `--fail-on-error` to opt into non-zero exit codes when files are skipped
- Added many new languages, bringing the total to 464
- Added support for shell completions with the `completions` subcommand
- Brang back `langs` as a subcommand, rather than a top-level flag
- Documented the code better
- Fixed a couple edgecases with whitespace classification
- Fixed the handling of shebangs with whitespace after the #!
- Improved UTF-16 detection without BOMs
- Updated dependencies to their latest versions

## 0.5.0

- Added a TOML config file
- Add JSON compact and TSV output formats
- Expand sorting, number formatting, size units, and precision controls
- Streamline CLI flags and improve output consistency
- Improve CSV and TSV escaping and blank field handling
- Add many new languages and updated benchmarks

## 0.4.0

- Rework display formatting to support sorting and custom output styles
- Add HTML and Markdown output formats using templates
- Remove the git dependency for gitignore handling
- Add more language fixtures and detection tests
- Improve documentation and CLI help formatting

## 0.3.1

- Add memory mapped I O for large files and improve comment parsing
- Switch pattern matching to globset and simplify language lookups
- Speed up core routines and reduce allocations
- Add more languages and fixes in the language data

## 0.3.0

- Switch language data to JSON5 with a cleaner build pipeline
- Add support for extensionless files with shebangs
- Improve output formatting and writer based rendering
- Expand language detection and add more tests

## 0.2.0

- Split the project into a library and binary crate and refactor the core
- Add stricter language data validation and cleanup for output
- Improve CLI flags and analyzer defaults
- Add many new languages and comment detection tweaks

## 0.1.0

- Initial release
