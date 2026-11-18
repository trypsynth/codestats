# Changelog

## Unreleased

- TeX/LaTeX claims LaTeX classes (`.cls`), definition files (`.def`), font definitions (`.fd`, `.fdd`), and l3build tests (`.lvt`, `.pvt`) that use its commands. They were counted as Visual Basic, Module Definition, or not at all: LaTeX's own source counts 1,771 more TeX files. PostScript claims dvips's `.pro` prologues, which were QMake
- Fixed Objective-C++ files without Objective-C in them being counted as Maple, Coq files being counted as V, and Metal shaders named `.msl` not being counted. Visual Basic's, QMake's, V's, Verilog's, and Coq's keywords match their case, so English prose doesn't match them
- Added strings to Erlang: OTP 27's triple-quoted strings, strings that span lines, and quoted atoms, so a `%` line in a `-doc """` example isn't a comment. OTP counts 601 more lines of code
- Fixed character literals holding a quote or a comment marker, like Smalltalk's and Erlang's `$"`, Erlang's `$%`, and Elixir's `?"`, opening a comment or a string that ran on
- Pharo's Tonel files count as Smalltalk without a `.gitattributes` to say so: 2,513 more of Pharo's files count
- Added strings to Starlark and Bazel, with docstrings that count as comment as Python's do, and to Meson, Gleam, CUE, KCL, Crystal, and Typst's raw text, so a comment marker inside one doesn't start a comment. bazel-skylib counts 1,359 more lines of comment, from its docstrings
- Fixed PHP 8 attributes such as `#[Route('/x')]` being counted as comments: Laravel counts 2,134 more lines of code
- KCL files without a `schema` are KCL, not K, and `kcl.mod` files are TOML
- Fixed strings that span lines in V, Tcl, Lean, and Elixir's `'''` heredocs, and V's raw strings, ending with their first line: V's standard library counts 6,977 more lines of code. A Tcl comment ending in `\` goes on to the next line, as `exec tclsh "$0"` restarts rely on
- Koka's, WGSL's, WESL's, and Typst's block comments nest
- Fixed the escapes in Nix's `''` strings, `''$`, `'''`, and `''\n`, ending the string: nixpkgs counts 27,287 more lines of code, and its `perl-packages.nix` counts 213 lines of comment, not 23,551
- GNU assembly's `#` and ARM's `@` open a comment in a line's first column, so `#----` banners and bare `#` lines aren't code. A directive like `#include` stays code, and `# error`, `# warning`, `# line`, `# elifdef`, and an `# else` with a comment after it count as directives. The Linux kernel's `.S` files count 3,965 more lines of comment
- Fixed Free Pascal's `.pp` and `.inc` files being counted as Puppet, C++, or BitBake, or not at all, when they held declarations without `begin`: 1,848 more of FPC's files count as Pascal. Puppet's and BitBake's keywords match their case now, so `{$DEFINE}` and "Description:" aren't theirs
- Added gforth's `.fs` to Forth, which was counted as F#, and a `\` that ends a line or comes before a tab opens a Forth comment too
- Fixed Octave files being counted as MATLAB, with every `#` comment as code, once they used MATLAB's keywords: the Octave statistics package's 777 files are all Octave now. MATLAB files are no longer taken for Miranda, Limbo, or Mercury
- In detection, each language looks for its keywords outside its own comments, so a C file's `// the interface` isn't Pascal's `interface`
- Fixed Idris's `|||` documentation comments being counted as code: Idris 2 counts 9,172 more lines of comment
- Julia's docstrings count as comment, as Python's do: Julia's own source counts 46,348 more lines of comment
- Fixed Nim's `##[ ]##` documentation blocks ending after their first line, and CoffeeScript's `####` headings opening a block comment, which made everything up to the next heading comment: CoffeeScript's compiler counts 1,584 more lines of code
- Added cram tests (`.t`), as dune and Mercurial write them, which were counted as Turing, Terra, TADS, or Perl, or not at all: dune's 2,276 count now. Their `  $ ` and `  > ` command lines count as code, in Bash, their expected output counts as code, and their prose counts as comment
- Fixed Raku's tests (`.t`) being counted as Perl: rakudo's 1,035 Raku files were 506. Raku's `=begin pod` blocks now end with their `=end` line, and `` #`(( )) `` comments with doubled brackets end at the doubled closer
- Fixed Perl POD that opens with `=item`, `=back`, `=end`, `=head5`, or `=head6` being counted as code. ExtUtils::MakeMaker, for one, counts 2,140 more lines of comment
- Fixed a `.t` file being taken for Turing, TADS, or Terra for a word like `get` or `put`, or for `*/*` in a MIME type, which looked like a `/* */` comment. Perl is the default for `.t` now, and the others need their own keywords. GHC's `all.T` test files count as Python
- Shebangs are recognized by their interpreter's name, wherever it's installed: `#!./perl`, `#!/usr/local/bin/ruby`, and `#!/usr/bin/env wish` all count, and perl5's tests are now Perl
- Added Jupyter notebooks (`.ipynb`), which weren't counted at all. Code cells count as code in the kernel's language, such as Python, R, or Julia, or in the language a cell magic like `%%bash` names, and show as that language's embedded code. Markdown and raw cells count as comment, and outputs aren't counted. Older notebooks, from nbformat 3, are read too. The fast.ai book's 44 notebooks, for one, now count 5,545 lines of Python
- Added `--show-skipped`, which reports the files that codestats leaves out as generated, vendored, or documentation, and why, with the lines each reason leaves out and the flags that would count them. Pass `-v` to list each file
- Vendored code is skipped, as GitHub skips it: files in `third_party`, `3rdparty`, `node_modules`, `bower_components`, `Pods`, and `Carthage` directories, and in a `vendor` directory beside `go.mod`, `composer.json`, `Gemfile`, or `Cargo.toml`. Pass `--include-vendored` to count them, along with files marked `linguist-vendored`. A file marked `-linguist-vendored` is counted wherever it is. kubernetes, for one, counts 4,437 fewer files and takes about 10% less time
- Added strings to 281 languages that had block comments but no strings, including D, CUDA, Solidity, Terraform, HCL, Verilog, VHDL, GLSL, Pascal, Haxe, Vala, Elm, CoffeeScript, Scheme, and Gradle, so a comment marker inside a string no longer starts a comment. Terraform and HCL heredocs are strings too. GHC's lexer, for one, now counts 2,346 more lines of code
- Added Literate Haskell (`.lhs`), Literate Agda (`.lagda`), and Literate Idris (`.lidr`), whose prose counts as comment and whose code, in lines starting with `>` or between `\begin{code}` and `\end{code}`, counts by Haskell's, Agda's, or Idris's rules. They were counted as all code
- Files whose header opens with "Generated by" or "Generated from", as Autoconf, automake, aclocal, Cython, pip-compile, and the Erlang ASN.1 compiler write, or says "this file was generated by", count as generated. Configure scripts and `_ssl_data` headers, for example, are now skipped unless you pass `--include-generated`
- Fixed npm's `npm-shrinkwrap.json` being counted: the generated name was listed as `shrinkwrap.json`. `pdm.lock`, `pixi.lock`, `mise.lock`, `MODULE.bazel.lock`, `Package.resolved`, `Cartfile.resolved`, `Gopkg.lock`, and Yarn's `.pnp.cjs` and `.pnp.loader.mjs` count as generated too
- JavaScript and CSS files over 4 KB whose lines average 500 bytes or more count as minified, and so as generated, whatever their name: a bundle like `dist/app.js` was counted as code unless it was named `.min.js`
- Added ProGuard, for `proguard-rules.pro` and `.pro` files that hold `-keep` rules, which were counted as QMake
- Fixed GLSL shaders named `.fs` being counted as F#, Google Apps Script's `.gs` files as Gosu, and Tcl scripts that start as shell scripts and restart with `exec tclsh` as Bash
- The binary is about 160 KB smaller, as it no longer depends on encoding_rs and num-format

## 1.1.2

- Fixed shell comments being found inside words, so `${#files[@]}`, `$#`, and `a#b` no longer hide the rest of their line, such as a heredoc opener
- Fixed quotes inside a shell `$(...)` in a double-quoted string ending that string, and `\'` ending a `$'...'` string. Many shell scripts now count more of their comments
- Fixed Haskell operators that start with `--`, such as `-->`, being read as comments
- Fixed F#'s `(*)` operator being read as the start of a block comment
- Fixed a `"` inside a Swift string written between `#"` and `"#` ending the string

## 1.1.1

- Writes reports faster and with less memory: `-v -o json` over 100,000 files went from 750 ms to 190 ms, and verbose human, CSV, and compact JSON reports take about 30% less time
- Counts large repositories faster: the Linux kernel's C files take about 10% less time and cpython's about 11% less. Lines are counted a block at a time, and files up to 256 KB are read instead of memory-mapped
- Builds on processors without AVX2 or NEON, such as RISC-V, where 1.1.0 failed to compile
- Fixed code after a heredoc's terminator on the same line being ignored, so a PHP `?>` there left the rest of the page counted as PHP

## 1.1.0

- Added `--no-config`, which reads no configuration file
- Added strings to Clojure, F#, and Odin, so a comment marker inside a multi-line string no longer counts as a comment
- Starts faster and uses about 1 MB less memory, which saves 10% on small repositories
- Translations and the files IDEs and interface designers save count as generated, so they're skipped unless you pass `--include-generated`: Gettext, XLIFF, `.resx`, `.adml`, Visual Studio solutions and filters, Visual Studio's own project files (not SDK-style ones), `.xib`, `.storyboard`, Qt Designer and Glade files, and IntelliJ, Eclipse, and ReSharper settings. `rebar.lock` also counts as generated
- Added 208 languages, including 1C Enterprise, Asymptote, bc, Dune, GAMS, Mermaid, MiniZinc, NetLogo, POV-Ray SDL, Simula, SourcePawn, SurrealQL, Toit, XS, Yul, and Zeek
- Added about 270 extensions and filenames for existing languages, including `.escript`, `.bats`, `.rbi`, `.sublime-settings`, `.containerfile`, `.tm`, and `.spacemacs`
- Added Raku's `` #`[ ``, `` #`{ ``, and `` #`< `` comments, longer Luau block comments, Cabal block comments, and GraphQL descriptions as comments
- A file that starts with an XML declaration, in UTF-8 or UTF-16, counts as XML even if its extension belongs to a code language, such as Qt's `.ts` translations
- `.pl` files with Prolog's `:-` count as Prolog, `.tpl` files with `{{` as Go Template, and `.cls` files with Apex's `public class` as Apex. Also added shared extensions for Beef (`.bf`), Hack (`.hh`), IDL (`.pro`), KCL (`.k`), Logos (`.x`), re2c (`.re`), and Roff (`.nr`)
- Fixed the first line of a reStructuredText code block being skipped when it starts with `:`

## 1.0.1

- A language name that doesn't exist in `--lang`, `--exclude-lang`, or a config file now prints a warning
- Markdown and HTML reports say how many languages or directories `--top-languages` and `--min-lines` left out
- Fixed "No recognized programming languages found" when `--min-lines` hides every language
- Fixed a line break in a file name splitting a row of a Markdown table
- Fixed "1 lines" and "Skipped 1 entries" in messages
- Reports on paths from standard input are titled "standard input" instead of "-" in human, Markdown, and HTML output
- Nested comments repeated on one long line no longer take quadratic time: a 600 KB line went from about 1 s to 5 ms

## 1.0.0

- Breaking: generated files are detected by their header (`Code generated ... DO NOT EDIT`, `@generated`, `<auto-generated />`, and similar) and skipped by default. Use `--include-generated` to count them
- Breaking: linguist attributes in `.gitattributes` files are respected: `linguist-generated`, `linguist-vendored`, `linguist-documentation`, and `linguist-language`. Use `--no-gitattributes` to ignore them
- Breaking: JSON reports, including `cs diff`'s, replace `path` with a `paths` list, and `schema_version` is now 2
- Breaking: the `size` column in CSV and TSV output is now `bytes`
- Breaking: unknown keys in a config file are now an error
- Breaking: `--top-languages 0` is now an error
- Count several paths together: `cs src tests build.rs`
- Added `--ext` (`include_extensions` in config files) to count only files with the given extensions
- `--lang` and `--exclude-lang` accept comma-separated lists
- Read the paths to count from standard input with `cs -`, for example `git ls-files -z | cs -`
- Config files accept a `paths` list
- Added Blueprint, JCL, Julius, Lucius, Mako, MLIR, Ren'Py, RPM Spec, Slang, Snakemake, Tact, TTCN-3, TypeSpec, Ur/Web, and Visual Studio Solution
- Added about 170 extensions and filenames, including `.sbt`, `config.ru`, `Vagrantfile`, `.bashrc`, `.zshrc`, C++ module extensions, GLSL and HLSL shader extensions, `.storyboard`, and `.xib`
- PHP files count the HTML outside `<?php ... ?>` as embedded HTML
- Strings are recognized in Nix, Julia, OCaml, Visual Basic, R, PowerShell, Protocol Buffers, Fish, and C3
- Interpolated strings are read correctly, including quotes and nested strings inside their holes, in C#, JavaScript, TypeScript, Ruby, Kotlin, Groovy, Dart, and Swift
- C++ raw strings with custom delimiters (`R"sql(...)sql"`), PowerShell here-strings, and CMake bracket comments (`#[==[ ... ]==]`) are recognized
- Fixed C3 vector types like `int[<*>]` opening a documentation comment
- Fixed an HTML `<script>` or `<style>` tag that never closes, and `<!--` inside a quoted attribute value
- Fixed Perl POD and MATLAB block comments starting or ending in the wrong places
- Fixed heredoc terminators' indentation rules, and Ruby's `class <<self` being read as a heredoc
- Fixed escaped quotes outside strings opening a string in shells and PowerShell
- Fixed strings ending in a run of quotes, like `""""`
- C line comments ending in a backslash continue onto the next line, and a string continued with a backslash ends at a blank line
- D files without a D keyword are counted
- `.spec`, `.target`, `.path`, `.pck`, and `.fnc` files from other tools are no longer counted as RPM Spec, systemd, or PL/SQL
- Fixed a hang on some tiny files containing a zero byte
- `--hidden` no longer counts files inside `.git`
- Fixed `cs completions` panicking when its output is cut short
- The summary's language list uses an Oxford comma
- Linux release binaries now link against glibc, with static musl builds still available, and all release binaries use profile-guided optimization
- Faster on most repositories, especially those with many strings or line comments, shared extensions, or generated files: about 30% on cli and rust, 35-40% on ripgrep, cpython, and Kubernetes, and 45% on Linux compared to 0.9.1

## 0.9.1

- Embedded code is broken down by language in every output format: an `embedded` list in JSON, and an embedded code section in the other formats
- Code blocks in AsciiDoc, Org, and reStructuredText are counted by their own language's rules
- HTML-based templates, such as ERB, Jinja2, Razor, and Twig, count their `<script>` and `<style>` elements by those elements' languages
- Heredocs are recognized in Bash, Zsh, C Shell, Perl, Ruby, Crystal, and PHP
- Bash and Zsh strings, TOML multi-line strings, and CMake quoted and bracket arguments are recognized
- `.dtso` files count as Device Tree, `.plist` as XML, and `makefile.vc`, `makefile.gcc`, `makefile.bcc`, and `makefile.unx` as Makefile
- More code block names are recognized, such as `shell`, `golang`, `elisp`, `csharp`, and `latex`
- Counting a single file with `--verbose` shows its name instead of an empty path
- About 15% faster on files with many comment lines or non-ASCII text, such as translations

## 0.9.0

- Breaking: JSON output has a new shape with a `schema_version` (1). Line counts are a `lines` object at every level, sizes are `bytes`, file counts are `file_count`, and per-file details are `files`. `summary` is now `totals`, `analysis_path` is `path`, a directory's `path` is its `name`, and `languages_hidden` and `dirs_hidden` are `hidden_languages` and `hidden_directories`. Percentages, averages, and `*_human` sizes are no longer in JSON
- Added `cs diff <FROM> [PATH]` to compare line counts by language between a git revision and the working tree, or another revision with `--to`
- Added `--git` (`git_tracked_only` in config files) to count only the files git tracks, including in submodules
- Code embedded in another language is counted by its own language's rules: `<script>` and `<style>` in HTML, Vue, Svelte, and Astro, Astro's frontmatter, and fenced code blocks and frontmatter in Markdown
- Xcode prefix headers (`.pch`) are counted
- A file of a format that can be a program, like a BRLTTY contraction table (`.ctb`), is counted as its shebang's language when it starts with one

## 0.8.4

- Added Texinfo, XAML, and liblouis and BRLTTY braille tables
- 6502 assembly (`.s65`, `.a65`) counts as Assembly
- Make dependency files (`.d`) are no longer counted as D
- `.tmpl` and `.gotxt` count as Go Template, `.vcproj` and `.vcxproj.filters` as MSBuild, and `configure.ac` and `configure.in` as M4
- `.c3t` files count as C3, and lowercase `sconscript` and `sconstruct` as Python
- 20-40% faster than 0.8.3

## 0.8.3

- `.h` files are counted as C++ when they use C++-only constructs, such as `namespace`, `template`, or `std::`, and as C otherwise
- Keywords inside comments no longer affect detection when the candidate languages share a comment syntax
- C, C++, Objective-C, and LPC keywords match case-sensitively
- About 40% faster on the Linux kernel and 17-30% faster on other large repositories

## 0.8.2

- `--by-dir` works in CSV, TSV, Markdown, and HTML output
- Fixed Markdown tables not rendering on GitHub
- Fixed `-q` with Markdown or HTML output printing "No recognized programming languages found."
- Fixed `--by-dir --sort-by files` sorting by line count
- `--min-lines` is applied before `--top-languages`
- Fixed binary files of small 16-bit values being counted as UTF-16 text
- Fixed an error when output is piped into a command that stops reading early, like `cs | head`
- Invalid option values name the option, list the accepted values, and suggest the closest one
- Files of a language excluded by `--lang` or `--exclude-lang` are skipped without being read

## 0.8.1

- JavaScript and TypeScript regex literals are recognized
- Shebangs using `env -S` are recognized, and shebangs match whole interpreter names
- Added shebangs for deno, tsx, ts-node, bun, Rscript, pwsh, swift, kotlin, osascript, sbcl, dash, make, and raku
- Improved detection for extensions shared by several languages
- `.inc` files can be C++, PHP, Makefile, or NASM assembly, and are only claimed with real evidence
- Assembly preprocessor directives written with a space (`# define`) are counted as code
- HTML and Markdown output are always included; the `html`, `markdown`, and `all-formats` features are removed

## 0.8.0

- Comment markers inside string literals are ignored in 29 languages, including C, C++, Java, Rust, Python, and JavaScript
- Python docstrings are counted as comments
- A block comment start inside a line comment no longer opens a block comment, and block comments only close with their own delimiter
- Fixed comment syntax for about 80 languages and which languages' block comments nest
- Improved detection for extensions shared by several languages, such as `.m` and `.h`
- Files with nothing distinctive no longer go uncounted when their extension is shared, and the most specific pattern wins
- Exact filenames are matched case-sensitively
- Fortran is split into free-form Fortran and Fortran (Fixed Form)
- Added Blade, GAP, Gnuplot, Go Template, Godot Resource, HTTP, Pawn, Smali, systemd, and Yuck, and more extensions and filenames
- Removed CSV, Jupyter Notebook, MessagePack, max, Piet, and Whitespace
- `-q` ends with a compact list of languages and their file counts
- About 7x faster startup, and faster language detection
- codestats is a binary-only crate; the library is no longer published

## 0.7.0

- Added `--by-dir` (`-D`) to show a breakdown by directory instead of by language; combining it with `--verbose` also lists individual files under each directory
- Added `--max-depth N` to cap directory traversal depth
- Added `--min-lines N` to hide entries with fewer than N total lines from the breakdown
- Added `--top-languages N` to limit the breakdown to the top N entries; the output notes how many were omitted when entries are hidden by either flag
- Expanded language coverage, now covering 544 total programming languages.
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
- Improved documentation and CLI help text
- Reworked display formatting to support sorting and custom output styles

## 0.3.1

- Added memory-mapped I/O for large files
- Added more languages and improved language data
- Improved comment parsing
- Sped up core routines

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
