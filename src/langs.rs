use phf::phf_map;

struct Language {
    name: &'static str,
    file_patterns: &'static [&'static str],
}

macro_rules! define_languages {
    ($( $name:expr => $patterns:expr ),* $(,)?) => {
        static LANGUAGES: phf::Map<&'static str, Language> = phf_map! {
            $(
                $name => Language {
                    name: $name,
                    file_patterns: $patterns,
                },
            )*
        };
    };
}

define_languages!(
    "Ada" => &["*.adb", "*.ads"],
    "AngelScript" => &["*.as", "*.angelscript"],
    "APL" => &["*.apl"],
    "AppleScript" => &["*.scpt", "*.applescript"],
    "Assembly" => &["*.asm", "*.s", "*.nasm"],
    "AsciiDoc" => &["*.adoc", "*.asciidoc"],
    "AutoHotkey" => &["*.ahk", "*.ahkl"],
    "AutoIt" => &["*.au3"],
    "AWK" => &["*.awk", "*.gawk", "*.nawk"],
    "BASIC" => &["*.bas", "*.bi"],
    "Batch Script" => &["*.bat", "*.cmd"],
    "Bazel" => &["BUILD", "BUILD.bazel", "WORKSPACE"],
    "BGT" => &["*.bgt"],
    "Brainfuck" => &["*.bf", "*.b"],
    "BSON" => &["*.bson"],
    "Buck" => &["BUCK"],
    "C" => &["*.c", "*.h"],
    "C++" =>
        &[
            "*.cpp", "*.c++", "*.cc", "*.cxx", "*.hpp", "*.h++", "*.hxx", "*.ino"
        ],
    "C#" => &["*.cs", "*.csx", "*.cake"],
    "Chapel" => &["*.chpl"],
    "Clojure" => &["*.clj", "*.cljs", "*.cljc", "*.edn"],
    "CMake" => &["*.cmake", "CMakeLists.txt"],
    "COBOL" => &["*.cbl", "*.cob", "*.cpy"],
    "CoffeeScript" => &["*.coffee"],
    "Crystal" => &["*.cr"],
    "CSON" => &["*.cson"],
    "CSS" => &["*.css", "*.sass", "*.scss"],
    "CSV" => &["*.csv"],
    "CUDA" => &["*.cu", "*.cuh"],
    "Cython" => &["*.pyx", "*.pxd", "*.pyi"],
    "D" => &["*.d", "*.di"],
    "Dart" => &["*.dart", "pubspec.yaml"],
        "Dockerfile" =>
        &[
            "*.dockerfile",
            "Dockerfile",
            "docker-compose.yml",
            "docker-compose.override.yml"
        ],
    "Eiffel" => &["*.e"],
    "EJS" => &["*.ejs"],
    "Elixir" => &["*.ex", "*.exs"],
    "Emacs Lisp" => &["*.el"],
        "Erlang" =>
        &["*.erl", "*.hrl", "rebar.config", "rebar.lock"],
    "F#" => &["*.fs", "*.fsi", "*.fsx"],
    "Forth" => &["*.4th", "*.fth", "*.frt"],
    "Fortran" => &["*.f", "*.for", "*.f90", "*.f95"],
        "GLSL" =>
        &[
            "*.glsl", "*.vert", "*.frag", "*.geom", "*.tesc", "*.tese", "*.comp"
        ],
    "Go" => &["*.go", "go.mod", "go.sum"],
    "Gradle" => &["*.gradle", "gradle.properties"],
    "Groovy" => &["*.groovy", "*.gvy", "*.gy", "*.gsh"],
    "Hack" => &["*.hh", "*.hhi", "*.hack"],
    "HAML" => &["*.haml"],
    "Handlebars" => &["*.hbs", "*.handlebars"],
    "Haskell" => &["*.hs", "*.lhs", ".ghci", "stack.yaml"],
    "HLSL" => &["*.hlsl", "*.fx", "*.fxh", "*.hlsli"],
    "HTML" => &["*.html", "*.htm", "*.xht", "*.xhtml"],
    "IDL" => &["*.idl", "*.widl"],
        "INI" =>
        &[
            "*.ini",
            "*.cfg",
            "*.prefs",
            "*.properties",
            ".editorconfig",
            ".gitconfig",
            "buildozer.spec"
        ],
    "Inno Setup" => &["*.iss"],
    "J" => &["*.ijs"],
    "Java" => &["*.java"],
    "Java Server Pages" => &["*.jsp"],
        "JavaScript" =>
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
        ],
    "JAWS Script" => &["*.jss", "*.jsh"],
    "Jinja2" => &["*.j2"],
    "JSON" => &["*.json"],
    "Julia" => &["*.jl"],
    "Kotlin" => &["*.kt", "*.kts"],
    "Less" => &["*.less"],
    "Lua" => &["*.lua", "*.wlua"],
        "Makefile" =>
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
        ],
        "Markdown" =>
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
        ],
    "Maven" => &["pom.xml"],
    "MesagePack" => &["*.msgpack"],
    "Meson" => &["meson.build"],
    "moo" => &["*.moo"],
    "Mustache" => &["*.mustache"],
    "Nim" => &["*.nim", "nim.cfg"],
    "NSIS" => &["*.nsi", "*.nsh"],
    "NVGT" => &["*.nvgt", ".nvgtrc"],
    "Objective-C" => &["*.m"],
    "Objective-C++" => &["*.mm"],
    "OCaml" => &["*.ml", "*.mli"],
    "Pascal" => &["*.pas", "*.pp", "*.p", "*.inc"],
        "PHP" =>
        &["*.php", "*.php3", "*.php4", "*.php5", "*.phps", "*.phpt"],
        "Perl" =>
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
        ],
    "Pony" => &["*.pony"],
    "PowerShell" => &["*.ps1", "*.psd1", "*.psm1"],
    "PureBasic" => &["*.pb", "*.pbi", "*.pbf", "*.pbp"],
        "Python" =>
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
        ],
    "R" => &["*.r", "*.rmd", ".Rprofile"],
    "Racket" => &["*.rkt"],
        "Raku" =>
        &[
            "*.raku",
            "*.rakumod",
            "*.rakutest",
            "*.pm6",
            "*.pl6",
            "*.p6"
        ],
    "Re-structured Text" => &["*.rst"],
        "Ruby" =>
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
        ],
    "Rust" => &["*.rs", "Cargo.toml", "Cargo.lock"],
    "Scala" => &["*.scala", "*.sc", "build.sbt"],
    "Scheme" => &["*.scm", "*.ss"],
        "Shell Script" =>
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
        ],
    "SpiderBasic" => &["*.sb", "*.sbi", "*.sbf", "*.sbp"],
    "SQL" => &["*.sql"],
    "Swift" => &["*.swift"],
    "Tcl" => &["*.tcl", "*.tk"],
    "Terraform" => &["*.tf", "*.hcl", "terraform.tfvars"],
    "TeX/LaTeX" => &["*.tex", "*.sty", ".latexmkrc"],
    "TOML" => &["*.toml"],
    "Txt2tags" => &["*.t2t"],
    "TypeScript" => &["*.ts", "*.tsx", "tsconfig.json"],
    "V" => &["*.v"],
    "Verilog" => &["*.v", "*.vh", "*.sv", "*.svh"],
    "VHDL" => &["*.vhd", "*.vhdl"],
    "Vim script" => &["*.vim"],
        "Visual Basic" =>
        &[
            "*.vb", "*.cls", "*.frm", "*.frx", "*.vba", "*.vbhtml", "*.vbs"
        ],
    "Windows Registry Entry" => &["*.reg"],
    "XML" => &["*.xml"],
    "WebAssembly" => &["*.wat", "*.wasm"],
    "YAML" => &["*.yaml", "*.yml", ".yamllint"],
    "Zig" => &["*.zig"],
);

fn detect_language(filename: &str) -> Option<&'static str> {
    LANGUAGES.values().find_map(|language| {
        language.file_patterns.iter().find_map(|&pattern| {
            if let Some(suffix) = pattern.strip_prefix('*') {
                (filename.ends_with(suffix)).then_some(language.name)
            } else if filename == pattern {
                Some(language.name)
            } else {
                None
            }
        })
    })
}
