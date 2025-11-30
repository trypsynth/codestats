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
Usage: `cs <command>`

### Commands:
* analyze: Analyze a directory or file for code statistics.
* langs: List all supported programming languages (currently 410 and counting).
* help: Print program or command help.

### Options:
* -h, --help: Print help
* -V, --version: Print version

### Analyze
Analyze a directory or file for code statistics.

Usage: cs analyze [OPTIONS] <PATH>

### Arguments
* <PATH>  The path to analyze. This can be either a directory (which will be recursively analyzed) or a single file. If a directory is provided, all supported source files within it will be analyzed

### Options
* -v, --verbose Enable verbose output.
* -i, --no-gitignore Do not respect .gitignore files.
* -H, --hidden Search hidden files and directories.
* -s, --symlinks Follow symbolic links and include their targets in the analysis. Use with caution as this can lead to infinite loops with circular symlinks.
* -n, --number-style <NUMBER_STYLE> Output number formatting style [default: plain] [possible values: plain, comma, underscore, space].
* -u, --size-units <SIZE_STYLE> Human-readable size units [default: binary] [possible values: binary, decimal].
* -p, --precision <PERCENT_PRECISION> Percentage precision [default: 1].
* -S, --sort-by <LANGUAGE_SORT> Sorting key for languages (and per-file details when verbose) [default: lines] [possible values: lines, code, comments, blanks, files, size, name].
* -d, --sort-dir <SORT_DIRECTION> Sorting direction [default: desc] [possible values: asc, desc].
* -o, --output <OUTPUT> Output format [default: human] [possible values: human, json, csv, markdown, html].
* -h, --help Print help.

### Langs
List all supported programming languages

Usage: `codestats langs`

### Options
* -h, --help  Print help

## License
Codestats is licensed under the [Zlib License](LICENSE).
