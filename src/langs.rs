use lazy_static::lazy_static;
use std::collections::HashMap;

struct Language {
    name: &'static str,
    extensions: &'static [&'static str],
    associated_filenames: Option<&'static [&'static str]>,
}

macro_rules! insert_language {
    ($map:expr, $name:expr, $extensions:expr, $associated_filenames:expr) => {
        $map.insert(
            $name,
            Language {
                name: $name,
                extensions: $extensions,
                associated_filenames: $associated_filenames,
            },
        );
    };
}

lazy_static! {
    static ref LANGUAGES: HashMap<&'static str, Language> = {
        let mut map = HashMap::new();
        insert_language!(map, "Ada", &["adb", "ads"], None);
        insert_language!(map, "AngelScript", &["as", "angelscript"], None);
        insert_language!(map, "APL", &["apl"], None);
        insert_language!(map, "AppleScript", &["scpt", "applescript"], None);
        insert_language!(map, "Assembly", &["asm", "s", "nasm"], None);
        insert_language!(map, "AsciiDoc", &["adoc", "asciidoc"], None);
        insert_language!(map, "AutoHotkey", &["ahk", "ahkl"], None);
        insert_language!(map, "AutoIt", &["au3"], None);
        insert_language!(map, "AWK", &["awk", "gawk", "nawk"], None);
        insert_language!(map, "BASIC", &["bas", "bi"], None);
        insert_language!(map, "Batch Script", &["bat", "cmd"], None);
        insert_language!(
            map,
            "Bazel",
            &[],
            Some(&["BUILD", "BUILD.bazel", "WORKSPACE"])
        );
        insert_language!(map, "BGT", &["bgt"], None);
        insert_language!(map, "Brainfuck", &["bf"], None);
        insert_language!(map, "BSON", &["bson"], None);
        insert_language!(map, "Buck", &[], Some(&["BUCK"]));
        insert_language!(map, "C", &["c", "h"], None);
        insert_language!(
            map,
            "C++",
            &["cpp", "c++", "cc", "cxx", "hpp", "h++", "hxx", "ino"],
            None
        );
        insert_language!(map, "C#", &["cs", "csx", "cake"], None);
        insert_language!(map, "Chapel", &["chpl"], None);
        insert_language!(map, "Clojure", &["clj", "cljs", "cljc", "edn"], None);
        insert_language!(map, "CMake", &["cmake"], Some(&["CMakeLists.txt"]));
        insert_language!(map, "COBOL", &["cbl", "cob", "cpy"], None);
        insert_language!(map, "CoffeeScript", &["coffee"], None);
        insert_language!(map, "Crystal", &["cr"], None);
        insert_language!(map, "CSON", &["cson"], None);
        insert_language!(map, "CSS", &["css", "sass", "scss"], None);
        insert_language!(map, "CSV", &["csv"], None);
        insert_language!(map, "CUDA", &["cu", "cuh"], None);
        insert_language!(map, "Cython", &["pyx", "pxd", "pyi"], None);
        insert_language!(map, "D", &["d", "di"], None);
        insert_language!(map, "Dart", &["dart"], Some(&["pubspec.yaml"]));
        insert_language!(
            map,
            "Dockerfile",
            &["dockerfile"],
            Some(&[
                "Dockerfile",
                "docker-compose.yml",
                "docker-compose.override.yml"
            ])
        );
        insert_language!(map, "EDN", &["edn"], None);
        insert_language!(map, "Eiffel", &["e"], None);
        insert_language!(map, "EJS", &["ejs"], None);
        insert_language!(map, "Elixir", &["ex", "exs"], None);
        insert_language!(map, "Emacs Lisp", &["el"], None);
        insert_language!(
            map,
            "Erlang",
            &["erl", "hrl"],
            Some(&["rebar.config", "rebar.lock"])
        );
        insert_language!(map, "F#", &["fs", "fsi", "fsx"], None);
        insert_language!(map, "Forth", &["4th", "fth", "frt"], None);
        insert_language!(map, "Fortran", &["f", "for", "f90", "f95"], None);
        insert_language!(
            map,
            "GLSL",
            &["glsl", "vert", "frag", "geom", "tesc", "tese", "comp"],
            None
        );
        insert_language!(map, "Go", &["go"], Some(&["go.mod", "go.sum"]));
        insert_language!(map, "Gradle", &["gradle"], Some(&["gradle.properties"]));
        insert_language!(map, "Groovy", &["groovy", "gvy", "gy", "gsh"], None);
        insert_language!(map, "Hack", &["hh", "hhi", "hack"], None);
        insert_language!(map, "HAML", &["haml"], None);
        insert_language!(map, "Handlebars", &["hbs", "handlebars"], None);
        insert_language!(
            map,
            "Haskell",
            &["hs", "lhs"],
            Some(&[".ghci", "stack.yaml"])
        );
        insert_language!(map, "HLSL", &["hlsl", "fx", "fxh", "hlsli"], None);
        insert_language!(map, "HTML", &["html", "htm", "xht", "xhtml"], None);
        insert_language!(map, "Interface Definition Language", &["idl", "widl"], None);
        insert_language!(
            map,
            "INI",
            &["ini", "cfg", "prefs", "properties"],
            Some(&[".editorconfig", ".gitconfig", "buildozer.spec"])
        );
        insert_language!(map, "Inno Setup", &["iss"], None);
        insert_language!(map, "J", &["ijs"], None);
        insert_language!(map, "Java", &["java"], None);
        insert_language!(map, "Java Server Pages", &["jsp"], None);
        insert_language!(
            map,
            "JavaScript",
            &[
                "js", "cjs", "vue", "jsx", "jscad", "jsfl", "mjs", "njs", "sjs", "ssjs", "xsjs",
                "xsjslib",
            ],
            Some(&[".babelrc", ".eslintrc", ".prettierc"])
        );
        insert_language!(map, "JAWS Script", &["jss", "jsh"], None);
        insert_language!(map, "Jinja2", &["j2"], None);
        insert_language!(map, "JSON", &["json"], None);
        insert_language!(map, "Julia", &["jl"], None);
        insert_language!(map, "Kotlin", &["kt", "kts"], None);
        insert_language!(map, "Less", &["less"], None);
        insert_language!(map, "Lua", &["lua", "wlua"], None);
        insert_language!(
            map,
            "Makefile",
            &["mak", "make", "mk", "mkfile"],
            Some(&[
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
            ])
        );
        insert_language!(
            map,
            "Markdown",
            &[
                "md", "markdown", "mdown", "mdwn", "mkd", "mkdn", "mkdown", "ronn", "workbook"
            ],
            None
        );
        insert_language!(map, "Maven", &[], Some(&["pom.xml"]));
        insert_language!(map, "MesagePack", &["msgpack"], None);
        insert_language!(map, "Meson", &[], Some(&["meson.build"]));
        insert_language!(map, "moo", &["moo"], None);
        insert_language!(map, "Mustache", &["mustache"], None);
        insert_language!(map, "Nim", &["nim"], Some(&["nim.cfg"]));
        insert_language!(map, "NSIS", &["nsi", "nsh"], None);
        insert_language!(map, "NVGT", &["nvgt"], Some(&[".nvgtrc"]));
        insert_language!(map, "Objective-C", &["m"], None);
        insert_language!(map, "Objective-C++", &["mm"], None);
        insert_language!(map, "OCaml", &["ml", "mli"], None);
        insert_language!(map, "Pascal", &["pas", "pp", "p", "inc"], None);
        insert_language!(
            map,
            "PHP",
            &["php", "php3", "php4", "php5", "phps", "phpt"],
            None
        );
        insert_language!(
            map,
            "Perl",
            &["pl", "al", "perl", "plx", "pm"],
            Some(&[
                "Makefile.pl",
                "rexfile",
                "ack",
                "cpanfile",
                "cpanfile.snapshot"
            ])
        );
        insert_language!(map, "Pony", &["pony"], None);
        insert_language!(map, "PowerShell", &["ps1", "psd1", "psm1"], None);
        insert_language!(map, "PureBasic", &["pb", "pbi", "pbf", "pbp"], None);
        insert_language!(
            map,
            "Python",
            &["py", "pyw", "py2", "py3", "pip"],
            Some(&[
                ".gclient",
                "SConscript",
                "SConstruct",
                "Snakefile",
                "requirements.txt",
                "pyproject.toml  ",
                "tox.ini  ",
                "Pipfile  ",
                "Pipfile.lock"
            ])
        );
        insert_language!(map, "R", &["r", "rmd"], Some(&[".Rprofile"]));
        insert_language!(map, "Racket", &["rkt"], None);
        insert_language!(
            map,
            "Raku",
            &["raku", "rakumod", "rakutest", "pm6", "pl6", "p6"],
            None
        );
        insert_language!(map, "Re-structured Text", &["rst"], None);
        insert_language!(
            map,
            "Ruby",
            &[
                "rb",
                "builder",
                "eye",
                "gemspec",
                "god",
                "jbuilder",
                "mspec",
                "pluginspec",
                "podspec",
                "rabl",
                "rake",
                "rbuild",
                "rbw",
                "rbx",
                "ruby",
                "thor",
                "watchr"
            ],
            Some(&[
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
            ])
        );
        insert_language!(map, "Rust", &["rs"], Some(&["Cargo.toml", "Cargo.lock"]));
        insert_language!(map, "Scala", &["scala", "sc"], Some(&["build.sbt"]));
        insert_language!(map, "Scheme", &["scm", "ss"], None);
        insert_language!(
            map,
            "Shell Script",
            &["sh", "bash", "ksh", "tmux", "zsh"],
            Some(&[
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
            ])
        );
        insert_language!(map, "SpiderBasic", &["sb", "sbi", "sbf", "sbp"], None);
        insert_language!(map, "SQL", &["sql"], None);
        insert_language!(map, "Swift", &["swift"], None);
        insert_language!(map, "Tcl", &["tcl", "tk"], None);
        insert_language!(
            map,
            "Terraform",
            &["tf", "hcl"],
            Some(&["terraform.tfvars"])
        );
        insert_language!(map, "TeX/LaTeX", &["tex", "sty"], Some(&[".latexmkrc"]));
        insert_language!(map, "TOML", &["toml"], None);
        insert_language!(map, "Txt2tags", &["t2t"], None);
        insert_language!(map, "TypeScript", &["ts", "tsx"], Some(&["tsconfig.json"]));
        insert_language!(map, "V", &["v"], None);
        insert_language!(map, "Verilog", &["v", "vh", "sv", "svh"], None);
        insert_language!(map, "VHDL", &["vhd", "vhdl"], None);
        insert_language!(map, "Vim script", &["vim"], None);
        insert_language!(
            map,
            "Visual Basic",
            &["vb", "cls", "frm", "frx", "vba", "vbhtml", "vbs"],
            None
        );
        insert_language!(map, "Windows Registry Entry", &["reg"], None);
        insert_language!(map, "XML", &["xml"], None);
        insert_language!(map, "WebAssembly", &["wat", "wasm"], None);
        insert_language!(map, "YAML", &["yaml", "yml"], Some(&[".yamllint"]));
        insert_language!(map, "Zig", &["zig"], None);
        map
    };
}
