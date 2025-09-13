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
Codestats for codestats: 13 files, 2543 total lines, 62.8 KiB total size.
Line breakdown: 2428 code lines, 28 comment lines, 87 blank lines
Percentages: 95.5% code, 1.1% comments, 3.4% blanks
Language breakdown:
JSON:
	Files: 1 file (7.7% of total).
	Lines: 1512 lines (59.5% of total).
	Size: 32.8 KiB (52.3% of total).
	Line breakdown:
		Code: 1511 lines (99.9%).
		Blanks: 1 lines (0.1%).
Rust:
	Files: 9 files (69.2% of total).
	Lines: 935 lines (36.8% of total).
	Size: 27.1 KiB (43.2% of total).
	Line breakdown:
		Code: 839 lines (89.7%).
		Comments: 28 lines (3.0%).
		Blanks: 68 lines (7.3%).
Markdown:
	Files: 1 file (7.7% of total).
	Lines: 51 lines (2.0% of total).
	Size: 1.5 KiB (2.4% of total).
	Line breakdown:
		Code: 36 lines (70.6%).
		Blanks: 15 lines (29.4%).
TOML:
	Files: 2 files (15.4% of total).
	Lines: 45 lines (1.8% of total).
	Size: 1.3 KiB (2.0% of total).
	Line breakdown:
		Code: 42 lines (93.3%).
		Blanks: 3 lines (6.7%).
```

And the same directory, but this time with verbose output.

```
Analyzing directory codestats
Codestats for codestats: 13 files, 2543 total lines, 62.8 KiB total size.
Line breakdown: 2428 code lines, 28 comment lines, 87 blank lines
Percentages: 95.5% code, 1.1% comments, 3.4% blanks
Language breakdown:
JSON:
	Files: 1 file (7.7% of total).
	Lines: 1512 lines (59.5% of total).
	Size: 32.8 KiB (52.3% of total).
	Line breakdown:
		Code: 1511 lines (99.9%).
		Blanks: 1 lines (0.1%).
	File breakdown:
		codestats\src\languages.json: 1512 lines (59.5% of total).
Rust:
	Files: 9 files (69.2% of total).
	Lines: 935 lines (36.8% of total).
	Size: 27.1 KiB (43.2% of total).
	Line breakdown:
		Code: 839 lines (89.7%).
		Comments: 28 lines (3.0%).
		Blanks: 68 lines (7.3%).
	File breakdown:
		codestats\build.rs: 263 lines (10.3% of total).
		codestats\src\analyzer.rs: 252 lines (9.9% of total).
		codestats\src\comments.rs: 149 lines (5.9% of total).
		codestats\src\stats.rs: 120 lines (4.7% of total).
		codestats\src\cli.rs: 42 lines (1.7% of total).
		codestats\src\langs.rs: 39 lines (1.5% of total).
		codestats\src\main.rs: 33 lines (1.3% of total).
		codestats\templates\languages.rs: 30 lines (1.2% of total).
		codestats\src\utils.rs: 7 lines (0.3% of total).
Markdown:
	Files: 1 file (7.7% of total).
	Lines: 51 lines (2.0% of total).
	Size: 1.5 KiB (2.4% of total).
	Line breakdown:
		Code: 36 lines (70.6%).
		Blanks: 15 lines (29.4%).
	File breakdown:
		codestats\README.md: 51 lines (2.0% of total).
TOML:
	Files: 2 files (15.4% of total).
	Lines: 45 lines (1.8% of total).
	Size: 1.3 KiB (2.0% of total).
	Line breakdown:
		Code: 42 lines (93.3%).
		Blanks: 3 lines (6.7%).
	File breakdown:
		codestats\Cargo.toml: 32 lines (1.3% of total).
		codestats\rustfmt.toml: 13 lines (0.5% of total).
```

## License
Codestats is licensed under the [Zlib License](LICENSE).
