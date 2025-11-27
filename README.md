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
* -n, --no-gitignore Do not respect .gitignore/.ignore and similar files.
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
Codestats for codestats: 18 files, 4435 total lines, 115 KiB total size.
Line breakdown: 4066 code lines, 160 comment lines, 209 blank lines.
Percentages: 91.7% code, 3.6% comments, 4.7% blanks.
Language breakdown:
Rust:
	Files: 14 files (77.8% of total).
	Lines: 2209 lines (49.8% of total).
	Size: 65.3 KiB (56.6% of total).
	Line breakdown:
		Code: 1860 lines (84.2%).
		Comments: 160 lines (7.2%).
		Blanks: 189 lines (8.6%).
JSON5:
	Files: 1 file (5.6% of total).
	Lines: 2079 lines (46.9% of total).
	Size: 45.8 KiB (39.7% of total).
	Line breakdown:
		Code: 2079 lines (100.0%).
Markdown:
	Files: 1 file (5.6% of total).
	Lines: 92 lines (2.1% of total).
	Size: 2.56 KiB (2.2% of total).
	Line breakdown:
		Code: 77 lines (83.7%).
		Blanks: 15 lines (16.3%).
TOML:
	Files: 2 files (11.1% of total).
	Lines: 55 lines (1.2% of total).
	Size: 1.61 KiB (1.4% of total).
	Line breakdown:
		Code: 50 lines (90.9%).
		Blanks: 5 lines (9.1%).
```

## License
Codestats is licensed under the [Zlib License](LICENSE).
