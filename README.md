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

## Supported programming languages
Codestats currently supports 356 unique programming languages, with more being added all the time. They are all listed below:

* 8th
* ABAP
* ABNF
* ActionScript
* Ada
* Agda
* Alex
* Alloy
* AngelScript
* Apache Config
* Apex
* APL
* AppleScript
* Arturo
* AsciiDoc
* ASN.1
* ASP
* ASP.NET
* Assembly
* Astro
* Ats
* Austral
* AutoHotkey
* AutoIt
* Automake
* AWK
* Ballerina
* Bash
* Batch Script
* Bazel
* BCX/FreeBASIC
* Bean
* Beef
* BGT
* BibTeX
* Bicep
* Bison
* BlitzBasic
* Boo
* BQN
* Brainfuck
* BrightScript
* Buck
* C
* C Shell
* C#
* C++
* C3
* Cabal
* Caddyfile
* Cairo
* Cangjie
* Carbon
* Cassius
* Ceylon
* ChaiScript
* Chapel
* ChucK
* Circom
* Clojure
* CMake
* COBOL
* Cobra
* CodeQL
* CoffeeScript
* Cogent
* ColdFusion
* Common Lisp
* Coq
* Crystal
* CSON
* CSS
* CSV
* CUDA
* CUE
* Cython
* D
* D2
* Dafny
* DAML
* Dart
* Delphi/Object Pascal
* Device Tree
* Dhall
* Dockerfile
* DotNet Resource
* Dream Maker
* Dust.js
* Dylan
* EBNF
* Ebuild
* EdgeQL
* Effekt
* Eiffel
* EJS
* Elixir
* Elm
* Elvish
* Emacs Dev Env
* Emacs Lisp
* EmberScript
* Emojicode
* Erlang
* Euphoria
* F#
* F*
* Factor
* Fantom
* Faust
* Felix
* Fennel
* Fish
* FlatBuffers Schema
* Forth
* Fortran
* FreeMarker
* Futhark
* GDB Script
* GDScript
* Gettext
* Gherkin
* Gleam
* GLSL
* GML
* Go
* Gosu
* Gradle
* Grain
* GraphQL
* Graphviz/DOT
* Groovy
* Gwion
* Hack
* Haml
* Hamlet
* Handlebars
* Happy
* Haskell
* Haxe
* HCL
* Headache
* HLSL
* HolyC
* HTML
* Hy
* Icon
* IDL
* Idris
* Inform 7
* INI
* Inno Setup
* Io
* Isabelle
* J
* Jai
* Janet
* Java
* Java Server Pages
* JavaScript
* JAWS Script
* Jinja2
* jq
* JSLT
* JSON
* JSONC
* Jsonnet
* Julia
* Jupyter Notebook
* Just
* K
* Kaitai Struct
* Kakoune script
* Koka
* Korn Shell
* Kotlin
* KV Language
* LALRPOP
* Lauterbach CMM
* LD Script
* Lean
* Less
* Lex
* LilyPond
* Lingua Franca
* Liquid Templates
* LiveScript
* LLVM IR
* Logtalk
* Lolcode
* Lua
* Luau
* M4
* Madlang
* Makefile
* Markdown
* Mathematica/Wolfram
* MATLAB
* Maven
* max
* Menhir
* Mercury
* Meson
* MessagePack
* Metal Shading Language
* Mint
* Mlatu
* Modelica
* Modula-2/3
* Module Definition
* Mojo
* Monkey C
* Moo
* MoonBit
* MoonScript
* Motoko
* Move
* MSBuild
* Mustache
* NASM
* Nearley
* Nemerle
* Nextflow
* Nginx Config
* Nim
* Ninja
* Nix
* Not Quite Perl
* NSIS
* NuGet Config
* Nunjucks Templates
* Nushell
* NVGT
* Oberon
* Objective-C
* Objective-C++
* Objective-J
* OCaml
* Odin
* Open Policy Agent
* OpenCL
* OpenQASM
* OpenSCAD
* OpenType Feature File
* Org
* Oz
* Pan
* Pascal
* Perl
* Pest
* PHP
* PicoLisp
* Pike
* Pine Script
* PKGBUILD
* PlantUML
* PogoScript
* Poke
* Pony
* PostCSS
* PostScript
* PowerShell
* Prisma
* Processing
* Prolog
* Promela
* Protocol Buffers
* Pug
* Puppet
* PureBasic
* PureScript
* Pyret
* Python
* Q
* Q#
* QCL
* QML
* R
* Racket
* Raku
* Rebol
* Red
* ReScript
* reStructuredText
* REXX
* Ring
* Robot Framework
* Roc
* Ruby
* Rust
* SageMath
* Sass
* Scala
* Scheme
* SCSS
* Sed
* Shen
* Slim
* Slint
* Smalltalk
* Smarty
* SNOBOL
* Solidity
* SpiderBasic
* SQF
* SQL
* Squirrel
* Stan
* Standard ML
* Stata
* Stylus
* SuperCollider
* Svelte
* SVG
* Swift
* Swig
* TC Shell
* Tcl
* Teal
* Templ
* Tera
* TeX/LaTeX
* Textile
* Thrift
* TLA+
* Tmux
* TOML
* Twig
* txt2tags
* TypeScript
* Umka
* Unison
* UnrealScript
* Uxntal
* V
* Vala
* Vale
* Verilog
* VHDL
* Vim script
* Visual Basic/Visual Basic .NET
* Vue
* WebAssembly
* WebIDL
* Wenyan
* WGSL
* Windows Registry Entry
* Windows Resource File
* WiX
* Wren
* XML
* Yacc
* YAML
* ZenCode
* Zig
* ZoKrates
* ZSH

## License
Codestats is licensed under the [Zlib License](LICENSE).
