# Codestats
This is a CLI tool written in Rust to provide detailed analysis about a folder containing source code, with features such as rspecting gitignores, following symlinks, ignoring hidden files, etc.

## Installation
### With cargo
```
cargo install codestats
```

### From source
```
git clone https://github.com/trypsynth/codestats
cd codestats
cargo install --path .
```

## Usage:
Usage: `codestats <command>`

Commands:
* analyze: Analyze a directory or file for code statistics.
* langs: List all supported programming languages.
* help: Print program or command help.

Options:
* -h, --help: Print help
* -V, --version: Print version

### Analyze
Analyze a directory or file for code statistics

Usage: `codestats analyze [OPTIONS] <PATH>`

Arguments:
* `<PATH>`: The path to analyze. This can be either a directory (which will be recursively analyzed) or a single file. If a directory is provided, all supported source files within it will be analyzed.

Options:
* -v, --verbose Enable verbose output.
* --no-gitignore Do not respect .gitignore/.ignore and similar files.
* --hidden Search hidden files and directories.
* -s, --symlinks Follow symlinks. When enabled, symbolic links will be followed and their targets will be included in the analysis. Use with caution as this can lead to infinite loops with circular symlinks.
* -h, --help Print help

### Langs
List all supported programming languages

Usage: `codestats langs`

Options:
* -h, --help  Print help

## Sample output
This is the result of running codestats on itself.

```
Codestats for codestats: 19 files, 4030 total lines, 101 KiB total size.
Line breakdown: 3674 code lines, 190 comment lines, 166 blank lines
Percentages: 91.2% code, 4.7% comments, 4.1% blanks
Language breakdown:
JSON:
	Files: 1 file (5.3% of total).
	Lines: 2023 lines (50.2% of total).
	Size: 44.1 KiB (43.6% of total).
	Line breakdown:
		Code: 2022 lines (100.0%).
		Blanks: 1 lines (0.0%).
Rust:
	Files: 15 files (78.9% of total).
	Lines: 1856 lines (46.1% of total).
	Size: 52.9 KiB (52.4% of total).
	Line breakdown:
		Code: 1521 lines (82.0%).
		Comments: 190 lines (10.2%).
		Blanks: 145 lines (7.8%).
Markdown:
	Files: 1 file (5.3% of total).
	Lines: 92 lines (2.3% of total).
	Size: 2.5 KiB (2.5% of total).
	Line breakdown:
		Code: 77 lines (83.7%).
		Blanks: 15 lines (16.3%).
TOML:
	Files: 2 files (10.5% of total).
	Lines: 59 lines (1.5% of total).
	Size: 1.5 KiB (1.5% of total).
	Line breakdown:
		Code: 54 lines (91.5%).
		Blanks: 5 lines (8.5%).
```

## License
Codestats is licensed under the [Zlib License](LICENSE).

