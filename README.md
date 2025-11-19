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
Codestats for .: 18 files, 4340 total lines, 110 KiB total size.
Line breakdown: 3994 code lines, 148 comment lines, 195 blank lines, 3 shebang lines.
Percentages: 92.0% code, 3.4% comments, 4.5% blanks, 0.1% shebangs.
Language breakdown:
Rust:
	Files: 14 files (77.8% of total).
	Lines: 2133 lines (49.1% of total).
	Size: 60.6 KiB (55.2% of total).
	Line breakdown:
		Code: 1808 lines (84.8%).
		Comments: 148 lines (6.9%).
		Blanks: 174 lines (8.2%).
		Shebangs: 3 lines (0.1%).
JSONC:
	Files: 1 file (5.6% of total).
	Lines: 2060 lines (47.5% of total).
	Size: 45.3 KiB (41.2% of total).
	Line breakdown:
		Code: 2060 lines (100.0%).
Markdown:
	Files: 1 file (5.6% of total).
	Lines: 93 lines (2.1% of total).
	Size: 2.54 KiB (2.3% of total).
	Line breakdown:
		Code: 77 lines (82.8%).
		Blanks: 16 lines (17.2%).
TOML:
	Files: 2 files (11.1% of total).
	Lines: 54 lines (1.2% of total).
	Size: 1.40 KiB (1.3% of total).
	Line breakdown:
		Code: 49 lines (90.7%).
		Blanks: 5 lines (9.3%).
```

## License
Codestats is licensed under the [Zlib License](LICENSE).
