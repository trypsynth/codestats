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
* -o, --output Choose how results are rendered. Options: `human` (default), `json`, `csv`, `markdown`.
* -h, --help Print help

### Langs
List all supported programming languages

Usage: `codestats langs`

Options:
* -h, --help  Print help

## Sample output
This is the result of running codestats on itself.

```
Codestats for .: 22 files, 4320 total lines, 113 KiB total size.
Line breakdown: 4006 code lines, 106 comment lines, 208 blank lines.
Percentages: 92.7% code, 2.5% comments, 4.8% blanks.
Language breakdown:
Rust:
	Files: 18 files (81.8% of total).
	Lines: 2331 lines (54.0% of total).
	Size: 65.5 KiB (58.0% of total).
	Line breakdown:
		Code: 2036 lines (87.3%).
		Comments: 106 lines (4.5%).
		Blanks: 189 lines (8.1%).
JSON5:
	Files: 1 file (4.5% of total).
	Lines: 1847 lines (42.8% of total).
	Size: 43.4 KiB (38.4% of total).
	Line breakdown:
		Code: 1847 lines (100.0%).
Markdown:
	Files: 1 file (4.5% of total).
	Lines: 91 lines (2.1% of total).
	Size: 2.51 KiB (2.2% of total).
	Line breakdown:
		Code: 76 lines (83.5%).
		Blanks: 15 lines (16.5%).
TOML:
	Files: 2 files (9.1% of total).
	Lines: 51 lines (1.2% of total).
	Size: 1.48 KiB (1.3% of total).
	Line breakdown:
		Code: 47 lines (92.2%).
		Blanks: 4 lines (7.8%).
```

## License
Codestats is licensed under the [Zlib License](LICENSE).
