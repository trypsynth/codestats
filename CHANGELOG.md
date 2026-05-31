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
