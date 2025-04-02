use lazy_static::lazy_static;
use std::collections::HashMap;

struct Language {
    name: &'static str,
    file_patterns: &'static [&'static str],
}

macro_rules! insert_language {
    ($map:expr, $name:expr, $patterns:expr) => {
        $map.insert(
            $name,
            Language {
                name: $name,
                file_patterns: $patterns,
            },
        );
    };
}

lazy_static! {
    static ref LANGUAGES: HashMap<&'static str, Language> = {
        let mut map = HashMap::new();
        insert_language!(map, "Ada", &["*.adb", "*.ads"]);
        insert_language!(map, "AngelScript", &["*.as", "*.angelscript"]);
        insert_language!(map, "APL", &["*.apl"]);
        insert_language!(map, "AppleScript", &["*.scpt", "*.applescript"]);
        insert_language!(map, "Assembly", &["*.asm", "*.s", "*.nasm"]);
        insert_language!(map, "AsciiDoc", &["*.adoc", "*.asciidoc"]);
        insert_language!(map, "AutoHotkey", &["*.ahk", "*.ahkl"]);
        insert_language!(map, "AutoIt", &["*.au3"]);
        insert_language!(map, "AWK", &["*.awk", "*.gawk", "*.nawk"]);
        insert_language!(map, "BASIC", &["*.bas", "*.bi"]);
        insert_language!(map, "Batch Script", &["*.bat", "*.cmd"]);
        insert_language!(map, "Bazel", &["BUILD", "BUILD.bazel", "WORKSPACE"]);
        insert_language!(map, "BGT", &["*.bgt"]);
        insert_language!(map, "Brainfuck", &["*.bf", "*.b"]);
        insert_language!(map, "BSON", &["*.bson"]);
        insert_language!(map, "Buck", &["BUCK"]);
        insert_language!(map, "C", &["*.c", "*.h"]);
        insert_language!(
            map,
            "C++",
            &[
                "*.cpp", "*.c++", "*.cc", "*.cxx", "*.hpp", "*.h++", "*.hxx", "*.ino"
            ]
        );
        insert_language!(map, "C#", &["*.cs", "*.csx", "*.cake"]);
        insert_language!(map, "Chapel", &["*.chpl"]);
        insert_language!(map, "Clojure", &["*.clj", "*.cljs", "*.cljc", "*.edn"]);
        insert_language!(map, "CMake", &["*.cmake", "CMakeLists.txt"]);
        insert_language!(map, "COBOL", &["*.cbl", "*.cob", "*.cpy"]);
        insert_language!(map, "CoffeeScript", &["*.coffee"]);
        insert_language!(map, "Crystal", &["*.cr"]);
        insert_language!(map, "CSON", &["*.cson"]);
        insert_language!(map, "CSS", &["*.css", "*.sass", "*.scss"]);
        insert_language!(map, "CSV", &["*.csv"]);
        insert_language!(map, "CUDA", &["*.cu", "*.cuh"]);
        insert_language!(map, "Cython", &["*.pyx", "*.pxd", "*.pyi"]);
        insert_language!(map, "D", &["*.d", "*.di"]);
        insert_language!(map, "Dart", &["*.dart", "pubspec.yaml"]);
        insert_language!(
            map,
            "Dockerfile",
            &[
                "*.dockerfile",
                "Dockerfile",
                "docker-compose.yml",
                "docker-compose.override.yml"
            ]
        );
        insert_language!(map, "Eiffel", &["*.e"]);
        insert_language!(map, "EJS", &["*.ejs"]);
        insert_language!(map, "Elixir", &["*.ex", "*.exs"]);
        insert_language!(map, "Emacs Lisp", &["*.el"]);
        insert_language!(
            map,
            "Erlang",
            &["*.erl", "*.hrl", "rebar.config", "rebar.lock"]
        );
        insert_language!(map, "F#", &["*.fs", "*.fsi", "*.fsx"]);
        insert_language!(map, "Forth", &["*.4th", "*.fth", "*.frt"]);
        insert_language!(map, "Fortran", &["*.f", "*.for", "*.f90", "*.f95"]);
        insert_language!(
            map,
            "GLSL",
            &[
                "*.glsl", "*.vert", "*.frag", "*.geom", "*.tesc", "*.tese", "*.comp"
            ]
        );
        insert_language!(map, "Go", &["*.go", "go.mod", "go.sum"]);
        insert_language!(map, "Gradle", &["*.gradle", "gradle.properties"]);
        insert_language!(map, "Groovy", &["*.groovy", "*.gvy", "*.gy", "*.gsh"]);
        insert_language!(map, "Hack", &["*.hh", "*.hhi", "*.hack"]);
        insert_language!(map, "HAML", &["*.haml"]);
        insert_language!(map, "Handlebars", &["*.hbs", "*.handlebars"]);
        insert_language!(map, "Haskell", &["*.hs", "*.lhs", ".ghci", "stack.yaml"]);
        insert_language!(map, "HLSL", &["*.hlsl", "*.fx", "*.fxh", "*.hlsli"]);
        insert_language!(map, "HTML", &["*.html", "*.htm", "*.xht", "*.xhtml"]);
        insert_language!(map, "IDL", &["*.idl", "*.widl"]);
        insert_language!(
            map,
            "INI",
            &[
                "*.ini",
                "*.cfg",
                "*.prefs",
                "*.properties",
                ".editorconfig",
                ".gitconfig",
                "buildozer.spec"
            ]
        );
        insert_language!(map, "Inno Setup", &["*.iss"]);
        insert_language!(map, "J", &["*.ijs"]);
        insert_language!(map, "Java", &["*.java"]);
        insert_language!(map, "Java Server Pages", &["*.jsp"]);
        insert_language!(
            map,
            "JavaScript",
            &[
                "*.js",
                "*.cjs",
                "*.vue",
                "*.jsx",
                "*.jscad",
                "*.jsfl",
                "*.mjs",
                "*.njs",
                "*.sjs",
                "*.ssjs",
                "*.xsjs",
                "*.xsjslib",
                ".babelrc",
                ".eslintrc",
                ".prettierc"
            ]
        );
        insert_language!(map, "JAWS Script", &["*.jss", "*.jsh"]);
        insert_language!(map, "Jinja2", &["*.j2"]);
        insert_language!(map, "JSON", &["*.json"]);
        insert_language!(map, "Julia", &["*.jl"]);
        insert_language!(map, "Kotlin", &["*.kt", "*.kts"]);
        insert_language!(map, "Less", &["*.less"]);
        insert_language!(map, "Lua", &["*.lua", "*.wlua"]);
        insert_language!(
            map,
            "Makefile",
            &[
                "*.mak",
                "*.make",
                "*.mk",
                "*.mkfile",
                "BSDmakefile",
                "GNUmakefile",
                "Kbuild",
                "Makefile",
                "Makefile.am",
                "Makefile.boot",
                "Makefile.frag",
                "Makefile.in",
                "Makefile.inc",
                "Makefile.wat",
                "makefile",
                "makefile.sco",
                "mkfile"
            ]
        );
        insert_language!(
            map,
            "Markdown",
            &[
                "*.md",
                "*.markdown",
                "*.mdown",
                "*.mdwn",
                "*.mkd",
                "*.mkdn",
                "*.mkdown",
                "*.ronn",
                "*.workbook"
            ]
        );
        insert_language!(map, "Maven", &["pom.xml"]);
        insert_language!(map, "MesagePack", &["*.msgpack"]);
        insert_language!(map, "Meson", &["meson.build"]);
        insert_language!(map, "moo", &["*.moo"]);
        insert_language!(map, "Mustache", &["*.mustache"]);
        insert_language!(map, "Nim", &["*.nim", "nim.cfg"]);
        insert_language!(map, "NSIS", &["*.nsi", "*.nsh"]);
        insert_language!(map, "NVGT", &["*.nvgt", ".nvgtrc"]);
        insert_language!(map, "Objective-C", &["*.m"]);
        insert_language!(map, "Objective-C++", &["*.mm"]);
        insert_language!(map, "OCaml", &["*.ml", "*.mli"]);
        insert_language!(map, "Pascal", &["*.pas", "*.pp", "*.p", "*.inc"]);
        insert_language!(
            map,
            "PHP",
            &["*.php", "*.php3", "*.php4", "*.php5", "*.phps", "*.phpt"]
        );
        insert_language!(
            map,
            "Perl",
            &[
                "*.pl",
                "*.al",
                "*.perl",
                "*.plx",
                "*.pm",
                "rexfile",
                "ack",
                "cpanfile",
                "cpanfile.snapshot"
            ]
        );
        insert_language!(map, "Pony", &["*.pony"]);
        insert_language!(map, "PowerShell", &["*.ps1", "*.psd1", "*.psm1"]);
        insert_language!(map, "PureBasic", &["*.pb", "*.pbi", "*.pbf", "*.pbp"]);
        insert_language!(
            map,
            "Python",
            &[
                "*.py",
                "*.pyw",
                "*.py2",
                "*.py3",
                "*.pip",
                ".gclient",
                "SConscript",
                "SConstruct",
                "Snakefile",
                "requirements.txt",
                "pyproject.toml  ",
                "tox.ini  ",
                "Pipfile  ",
                "Pipfile.lock"
            ]
        );
        insert_language!(map, "R", &["*.r", "*.rmd", ".Rprofile"]);
        insert_language!(map, "Racket", &["*.rkt"]);
        insert_language!(
            map,
            "Raku",
            &[
                "*.raku",
                "*.rakumod",
                "*.rakutest",
                "*.pm6",
                "*.pl6",
                "*.p6"
            ]
        );
        insert_language!(map, "Re-structured Text", &["*.rst"]);
        insert_language!(
            map,
            "Ruby",
            &[
                "*.rb",
                "*.builder",
                "*.eye",
                "*.gemspec",
                "*.god",
                "*.jbuilder",
                "*.mspec",
                "*.pluginspec",
                "*.podspec",
                "*.rabl",
                "*.rake",
                "*.rbuild",
                "*.rbw",
                "*.rbx",
                "*.ruby",
                "*.thor",
                "*.watchr",
                ".irbrc",
                ".pryrc",
                "Appraisals",
                "Berksfile",
                "Brewfile",
                "Buildfile",
                "Capfile",
                "Dangerfile",
                "Deliverfile",
                "Fastfile",
                "Gemfile",
                "Gemfile.lock",
                "Guardfile",
                "Jarfile",
                "Mavenfile",
                "Podfile",
                "Puppetfile",
                "Rakefile",
                "Snapfile",
                "Thorfile"
            ]
        );
        insert_language!(map, "Rust", &["*.rs", "Cargo.toml", "Cargo.lock"]);
        insert_language!(map, "Scala", &["*.scala", "*.sc", "build.sbt"]);
        insert_language!(map, "Scheme", &["*.scm", "*.ss"]);
        insert_language!(
            map,
            "Shell Script",
            &[
                "*.sh",
                "*.bash",
                "*.ksh",
                "*.tmux",
                "*.zsh",
                ".bash_logout",
                ".bash_profile",
                ".bashrc",
                ".login",
                ".profile",
                ".zlogin",
                ".zlogout",
                ".zprofile",
                ".zshenv",
                ".zshrc"
            ]
        );
        insert_language!(map, "SpiderBasic", &["*.sb", "*.sbi", "*.sbf", "*.sbp"]);
        insert_language!(map, "SQL", &["*.sql"]);
        insert_language!(map, "Swift", &["*.swift"]);
        insert_language!(map, "Tcl", &["*.tcl", "*.tk"]);
        insert_language!(map, "Terraform", &["*.tf", "*.hcl", "terraform.tfvars"]);
        insert_language!(map, "TeX/LaTeX", &["*.tex", "*.sty", ".latexmkrc"]);
        insert_language!(map, "TOML", &["*.toml"]);
        insert_language!(map, "Txt2tags", &["*.t2t"]);
        insert_language!(map, "TypeScript", &["*.ts", "*.tsx", "tsconfig.json"]);
        insert_language!(map, "V", &["*.v"]);
        insert_language!(map, "Verilog", &["*.v", "*.vh", "*.sv", "*.svh"]);
        insert_language!(map, "VHDL", &["*.vhd", "*.vhdl"]);
        insert_language!(map, "Vim script", &["*.vim"]);
        insert_language!(
            map,
            "Visual Basic",
            &[
                "*.vb", "*.cls", "*.frm", "*.frx", "*.vba", "*.vbhtml", "*.vbs"
            ]
        );
        insert_language!(map, "Windows Registry Entry", &["*.reg"]);
        insert_language!(map, "XML", &["*.xml"]);
        insert_language!(map, "WebAssembly", &["*.wat", "*.wasm"]);
        insert_language!(map, "YAML", &["*.yaml", "*.yml", ".yamllint"]);
        insert_language!(map, "Zig", &["*.zig"]);
        map
    };
}

fn detect_language(filename: &str) -> Option<&'static str> {
    for language in LANGUAGES.values() {
        if language.file_patterns.iter().any(|&pattern| {
            if let Some(filename) = pattern.strip_prefix('*') {                                                                 
            filename.ends_with(&pattern[1..])
            } else {
                filename == pattern
            }
        }) {
            return Some(language.name);
        }
    }
    None
}
