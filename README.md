# Codestats
Codestats is a command-line tool that provides a per-language breakdown of source code in a directory or file. It supports customizable analysis behavior, including `.gitignore` support, hidden file filtering, and symlink traversal.

## Usage
```sh
codestats [OPTIONS] <PATH>
```

### Arguments
* `<PATH>`
  The path to analyze. This can be either a single file or a directory. If a directory is provided, all supported source files within it will be recursively analyzed.

## Options
| Option               | Description                                                                                                |                                                                                                        |
| -------------------- | ---------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------ |
| `-v`, `--verbose`    | Enable verbose output, displaying detailed information about file processing.                              |                                                                                                        |
| \`--gitignore \<true | false>\`                                                                                                   | Respect `.gitignore` and `.ignore` files. Defaults to `true`. Use `--gitignore false` to disable.      |
| \`--hidden \<true    | false>\`                                                                                                   | Ignore hidden files and directories. Defaults to `true`. Use `--hidden false` to include hidden files. |
| `-s`, `--symlinks`   | Follow symbolic links. Useful for analyzing symlinked directories, but may introduce loops if not careful. |                                                                                                        |
| `-h`, `--help`       | Show help message.                                                                                         |                                                                                                        |
| `-V`, `--version`    | Show version information.                                                                                  |                                                                                                        |

---

## Examples

Analyze a directory with default settings:

```sh
codestats ./src
```

Disable `.gitignore` filtering:

```sh
codestats --gitignore false ./src
```

Include hidden files and follow symlinks:

```sh
codestats --hidden false --symlinks ./src
```

Verbose mode for debugging:

```sh
codestats --verbose ./src
```

Analyze a single file:

```sh
codestats ./main.cpp
```

---

## Building

```sh
cargo build --release
```

The compiled binary will be located at:

```sh
target/release/codestats
```

---

## License

This project is licensed under the MIT License.

---

Let me know if you'd like to include a section on supported languages, contributing, or installation via `cargo install`.
