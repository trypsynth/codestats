use phf::phf_map;

/// Holds information about a programming language, such as its name and associated file patterns.
///
/// File patterns can be wildcards (e.g. `*.cs`), or full filenames (e.g. `makefile`).
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
    "ABAP" => &["*.abap"],
    "Ada" => &["*.ada", "*.adb", "*.ads"],
    "Agda" => &["*.agda", "*.lagda"],
    "Alloy" => &["*.als"],
    "AngelScript" => &["*.as", "*.angelscript"],
    "APL" => &["*.apl", "*.dyalog"],
    "AppleScript" => &["*.scpt", "*.applescript"],
    "Assembly" => &["*.asm", "*.s", "*.nasm"],
    "AsciiDoc" => &["*.adoc", "*.asciidoc", "*.asc"],
    "AutoHotkey" => &["*.ahk", "*.ahkl", "*.ah2"],
    "AutoIt" => &["*.au3"],
    "AWK" => &["*.awk", "*.gawk", "*.nawk"],
    "Ballerina" => &["*.bal"],
    "BCX/FreeBASIC" => &["*.bas", "*.bi"],
    "Batch Script" => &["*.bat", "*.cmd"],
    "Bazel" => &["*.bzl", "*.star", "BUILD", "BUILD.bazel", "WORKSPACE", "MODULE.bazel"],
    "BGT" => &["*.bgt"],
    "Bicep" => &["*.bicep"],
    "BlitzBasic" => &["*.bb", "*.decls", "*.bb2"],
    "Boo" => &["*.boo"],
    "Brainfuck" => &["*.bf", "*.b"],
    "BrightScript" => &["*.brs"],
    "BSON" => &["*.bson"],
    "Buck" => &["BUCK"],
    "C" => &["*.c", "*.h"],
    "C++" =>
        &[
            "*.cpp", "*.hpp", "*.c++", "*.h++", "*.cc", "*.cxx", "*.hxx", "*.ino", "*.ipp", "*.pde", "*.cppm", "*.ixx", "*.c++m"
        ],
    "C#" => &["*.cs", "*.csx", "*.cake"],
    "Cabal" => &["*.cabal"],
    "ChaiScript" => &["*.chai"],
    "Chapel" => &["*.chpl"],
    "Clojure" => &["*.clj", "*.cljs", "*.cljc", "*.edn"],
    "CMake" => &["*.cmake", "*.cmake.in", "*.cmake.rule", "CMakeLists.txt"],
    "COBOL" => &["*.cbl", "*.cob", "*.cpy"],
    "CoffeeScript" => &["*.coffee"],
    "ColdFusion" => &["*.cfm", "*.cfc"],
    "Common Lisp" => &["*.lisp", "*.asd", "*.cl"],
    "Crystal" => &["*.cr"],
    "CSON" => &["*.cson"],
    "CSS" => &["*.css", "*.sass", "*.scss", "*.postcss", "*.styl"],
    "CSV" => &["*.csv"],
    "CUDA" => &["*.cu", "*.cuh", "*.ptx"],
    "Cython" => &["*.pyx", "*.pxd", "*.pyi"],
    "D" => &["*.d", "*.di"],
    "Dart" => &["*.dart"],
        "Dockerfile" =>
        &[
            "*.dockerfile",
            "Dockerfile",
            "docker-compose.yml",
            "docker-compose.override.yml"
        ],
    "EBNF" => &["*.ebnf"],
    "Eiffel" => &["*.e"],
    "EJS" => &["*.ejs"],
    "Elixir" => &["*.ex", "*.exs"],
    "Elm" => &["*.elm"],
    "Emacs Lisp" => &["*.el"],
    "EmberScript" => &["*.em"],
        "Erlang" =>
        &["*.erl", "*.hrl", "rebar.config", "rebar.lock"],
    "F#" => &["*.fs", "*.fsi", "*.fsx"],
    "Forth" => &["*.4th", "*.fth", "*.frt"],
    "Fortran" => &["*.f", "*.for", "*.f77", "*.f90", "*.f95", "*.f03"],
    "GDScript" => &["*.gd"],
    "GLSL" =>
        &[
            "*.glsl", "*.vert", "*.frag", "*.geom", "*.tesc", "*.tese", "*.comp"
        ],
    "Go" => &["*.go", "*.tmpl", "*.gohtml", "*.gotmpl", "go.mod"],
    "GraphQL" => &["*.graphql", "*.gql"],
    "Gradle" => &["*.gradle", "gradle.properties"],
    "Groovy" => &["*.groovy", "*.gvy", "*.gy", "*.gsh"],
    "Hack" => &["*.hh", "*.hhi", "*.hack"],
    "HAML" => &["*.haml"],
    "Handlebars" => &["*.hbs", "*.handlebars"],
    "Haskell" => &["*.hs", "*.lhs", "*.hsc", "*.hs-boot", ".ghci", "stack.yaml"],
    "HLSL" => &["*.hlsl", "*.fx", "*.fxh", "*.hlsli"],
    "HTML" => &["*.html", "*.htm", "*.xht", "*.xhtml"],
    "IDL" => &["*.idl", "*.widl"],
    "Inform 7" => &["*.ni", "*.i7x"],
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
    "Inno Setup" => &["*.iss", "*.ish"],
    "J" => &["*.ijs"],
    "Java" => &["*.java", "*.jav"],
    "Java Server Pages" => &["*.jsp"],
    "JavaScript" =>
        &[
            "*.js",
            "*.cjs",
            "*.jsx",
            "*.jscad",
            "*.jsfl",
            "*.mjs",
            "*.njs",
            "*.sjs",
            "*.ssjs",
            "*.xsjs",
            "*.xsjslib",
            "*.es",
            "*.es6",
            "*.jake",
            ".babelrc",
            ".eslintrc",
            ".prettierc"
        ],
    "JAWS Script" => &["*.jss", "*.jsh"],
    "Jinja2" => &["*.j2", "*.jinja", "*.jinja2"],
    "JSON" => &["*.json", "*.geojson", "*.jsonc", "*.jsonl", "*.ndjson"],
    "Julia" => &["*.jl"],
    "Kotlin" => &["*.kt", "*.kts", "*.ktm"],
    "Less" => &["*.less"],
    "LilyPond" => &["*.ly", "*.ily"],
    "Liquid Templates" => &["*.liquid"],
    "Lua" => &["*.lua", "*.wlua", "*.luau", "*.rockspec", ".luacheckrc"],
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
            "*.mdx",
            "*.mdown",
            "*.mdwn",
            "*.mkd",
            "*.mkdn",
            "*.mkdown",
            "*.ronn",
            "*.workbook"
        ],
    "Maven" => &["pom.xml"],
    "MessagePack" => &["*.msgpack"],
    "Meson" => &["meson.build"],
    "Modula-2/3" => &["*.m3", "*.mi", "*.mod"],
    "Moo" => &["*.moo"],
    "Mustache" => &["*.mustache"],
    "Nim" => &["*.nim", "nim.cfg"],
    "Ninja" => &["build.ninja"],
    "NSIS" => &["*.nsi", "*.nsh"],
    "Nunjucks Templates" => &["*.njk"],
    "NVGT" => &["*.nvgt", ".nvgtrc"],
    "Objective-C" => &["*.m"],
    "Objective-C++" => &["*.mm"],
    "Objective-J" => &["*.j", "*.sj"],
    "OCaml" => &["*.ml", "*.mli", "*.cmx", "*.cmxa", "*.mll", "*.mly"],
    "OpenSCAD" => &["*.scad"],
    "Oz" => &["*.oz"],
    "Pascal" => &["*.pas", "*.pp", "*.p", "*.inc"],
    "PHP" =>
        &["*.php", "*.php3", "*.php4", "*.php5", "*.phps", "*.phpt", "*.phtml"],
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
    "PogoScript" => &["*.pogo"],
    "Pony" => &["*.pony"],
    "PowerShell" => &["*.ps1", "*.psd1", "*.psm1", "*.ps1xml"],
    "Protocol Buffers" => &["*.proto"],
    "Pug" => &["*.pug", "*.jade"],
    "PureBasic" => &["*.pb", "*.pbi", "*.pbf", "*.pbp"],
        "Python" =>
        &[
            "*.py",
            "*.pyw",
            "*.py2",
            "*.py3",
            "*.pip", "*.pyz", "*.pyzw",
            ".gclient",
            "SConscript",
            "SConstruct",
            "Snakefile",
            "requirements.txt",
            "Pipfile",
            ".pythonrc", "py.typed",
        ],
    "Q#" => &["*.qs"],
    "QML" => &["*.qml"],
    "R" => &["*.r", "*.rmd", ".Rprofile"],
    "Racket" => &["*.rkt", "*.rktd", "*.rktl", "*.rktm"],
        "Raku" =>
        &[
            "*.raku",
            "*.rakumod",
            "*.rakutest",
            "*.pm6",
            "*.pl6",
            "*.p6"
        ],
    "reStructuredText" => &["*.rst"],
    "Robot Framework" => &["*.robot"],
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
            "Guardfile",
            "Jarfile",
            "Mavenfile",
            "Podfile",
            "Puppetfile",
            "Rakefile",
            "Snapfile",
            "Thorfile"
        ],
    "Rust" => &["*.rs", "*.rs.in", "*.rslib", "*.rlib"],
    "SageMath" => &["*.sage", "*.sagews"],
    "Scala" => &["*.scala", "*.sc", "build.sbt"],
    "Scheme" => &["*.scm", "*.ss"],
    "Sed" => &["*.sed"],
        "Shell Script" =>
        &[
            "*.sh",
            "*.bash",
            "*.ksh",
            "*.tmux",
            "*.zsh",
            "*.fish", "*.csh", "*.tcsh",
            ".bash_logout",
            ".bash_profile",
            ".bashrc",
            ".login",
            ".profile",
            ".zlogin",
            ".zlogout",
            ".zprofile",
            ".zshenv",
            ".zshrc",
            ".bash_aliases"
        ],
    "Slim" => &["*.slim"],
    "Smalltalk" => &["*.st"],
    "Smarty" => &["*.tpl"],
    "SpiderBasic" => &["*.sb", "*.sbi", "*.sbf", "*.sbp"],
    "SQL" => &["*.sql"],
    "Squirrel" => &["*.nut"],
    "Stan" => &["*.stan"],
    "Svelte" => &["*.stelte"],
    "Swift" => &["*.swift", "*.swiftpm"],
    "Tcl" => &["*.tcl", "*.tk"],
    "Terraform" => &["*.tf", "*.hcl", "terraform.tfvars"],
    "TeX/LaTeX" => &["*.tex", "*.sty", ".latexmkrc"],
    "Textile" => &["*.textile"],
    "TOML" => &["*.toml"],
    "txt2tags" => &["*.t2t"],
    "Twig" => &["*.twig"],
    "TypeScript" => &["*.ts", "*.tsx", "*.cts", "*.mts"],
    "UnrealScript" => &["*.uc"],
    "Vala" => &["*.vala", "*.vapi"],
    "Verilog" => &["*.v", "*.vh", "*.sv", "*.svh"],
    "VHDL" => &["*.vhd", "*.vhdl"],
    "Vim script" => &["*.vim"],
        "Visual Basic/Visual Basic .NET" =>
        &[
            "*.vb", "*.cls", "*.frm", "*.frx", "*.vba", "*.vbhtml", "*.vbs"
        ],
    "Vue" => &["*.vue"],
    "Windows Resource File" => &["*.rc"],
    "Windows Registry Entry" => &["*.reg"],
    "WiX" => &["*.wxs"],
    "XML" => &["*.xml", "*.svg"],
    "WebAssembly" => &["*.wat", "*.wasm"],
    "YAML" => &["*.yaml", "*.yml", "*.yaml.tmpl", "*.yaml-tmlp", ".yamllint"],
    "Zig" => &["*.zig"],
);

pub fn detect_language(filename: &str) -> Option<&'static str> {
    LANGUAGES.values().find_map(|language| {
        language.file_patterns.iter().find_map(|&pattern| {
            pattern.strip_prefix('*').map_or_else(
                || {
                    if filename == pattern {
                        Some(language.name)
                    } else {
                        None
                    }
                },
                |suffix| filename.ends_with(suffix).then_some(language.name),
            )
        })
    })
}
