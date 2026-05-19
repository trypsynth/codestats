# Changelog

## 0.7.0

- Added `--by-dir` (`-D`) to show a breakdown by directory instead of by language
- Added verbosity control via `-q`/`--quiet` (totals only) and `-v`/`--verbose` (per-file details); default shows the language breakdown
- Renamed `--sort-dir` to `--sort-direction`; dropped the `-i` shorthand on `--no-gitignore` and `-S` on `--symlinks`
- Added `--top-languages N` to limit the language breakdown to the top N languages
- Generated files (lockfiles and minified assets) are now excluded by default; pass `--include-generated` to count them
- Added `--max-depth N` to limit directory traversal depth
- Added `--min-lines N` to hide languages with fewer than N total lines from the breakdown
- Fixed unsupported block comment definitions for MoonScript

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
