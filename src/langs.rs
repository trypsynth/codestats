use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Represents a single programming language.
#[derive(Debug, Deserialize, Serialize)]
pub struct Language {
    /// Name of the language (e.g. `Rust`).
    pub name: String,
    /// A list of recognized file patterns for this language (e.g. `*.rs`, `makefile`).
    pub file_patterns: Vec<String>,
    /// A list of character sequences (if any) to be interpreted as single-line comments.
    pub line_comments: Option<Vec<String>>,
    /// A nested list of character sequences (if any) to be interpreted as block-style comments.
    pub block_comments: Option<Vec<Vec<String>>>,
    /// Does this programming language support nested block comments?
    pub nested_blocks: Option<bool>,
}

static LANGUAGES_JSON: &str = include_str!("languages.json");
static LANGUAGES: OnceLock<Vec<Language>> = OnceLock::new();

/// Returns a static reference to the deserialized language data, as loaded from `languages.json`.
fn get_languages() -> &'static Vec<Language> {
    LANGUAGES.get_or_init(|| {
        serde_json::from_str(LANGUAGES_JSON).expect("Failed to parse embedded languages.json")
    })
}

/// Tries to detect a programming language given a filename.
#[must_use]
pub fn detect_language(filename: &str) -> Option<String> {
    get_languages().iter().find_map(|language| {
        language.file_patterns.iter().find_map(|pattern| {
            pattern.strip_prefix('*').map_or_else(
                || {
                    if filename == pattern {
                        Some(language.name.clone())
                    } else {
                        None
                    }
                },
                |suffix| {
                    if filename.ends_with(suffix) {
                        Some(language.name.clone())
                    } else {
                        None
                    }
                },
            )
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language_known_extensions() {
        let cases = vec![
            ("test.rs", Some("Rust")),
            ("main.py", Some("Python")),
            ("script.js", Some("JavaScript")),
            ("index.html", Some("HTML")),
            ("style.css", Some("CSS")),
            ("main.cpp", Some("C++")),
            ("hello.c", Some("C")),
            ("program.java", Some("Java")),
            ("module.go", Some("Go")),
            ("unknownfile.xyz", None),
        ];
        for (input, expected) in cases {
            let detected = detect_language(input);
            assert_eq!(
                detected.as_deref(),
                expected,
                "Expected {:?} for file {:?}, got {:?}",
                expected,
                input,
                detected
            );
        }
    }

    #[test]
    fn test_detect_language_exact_match_special_filenames() {
        let cases = vec![
            ("Makefile", Some("Makefile")),
            ("Dockerfile", Some("Dockerfile")),
            ("Rakefile", Some("Ruby")),
            ("build.gradle", Some("Gradle")),
            ("CMakeLists.txt", Some("CMake")),
        ];
        for (input, expected) in cases {
            let detected = detect_language(input);
            assert_eq!(
                detected.as_deref(),
                expected,
                "Expected {:?} for special file {:?}, got {:?}",
                expected,
                input,
                detected
            );
        }
    }

    #[test]
    fn test_detect_language_case_sensitivity() {
        let cases = vec![
            ("makefile", Some("Makefile")),
            ("MAKEFILE", None),
            ("Dockerfile", Some("Dockerfile")),
        ];
        for (input, expected) in cases {
            let detected = detect_language(input);
            assert_eq!(
                detected.as_deref(),
                expected,
                "Expected {:?} for case test file {:?}, got {:?}",
                expected,
                input,
                detected
            );
        }
    }

    #[test]
    fn test_detect_language_edge_cases() {
        let cases = vec![
            ("", None),
            (".hidden", None),
            ("weird.file.name.rs", Some("Rust")),
            ("tricky.rs.txt", None),
        ];
        for (input, expected) in cases {
            let detected = detect_language(input);
            assert_eq!(
                detected.as_deref(),
                expected,
                "Expected {:?} for edge case file {:?}, got {:?}",
                expected,
                input,
                detected
            );
        }
    }

    #[test]
    fn test_languages_json_is_valid() {
        let langs = get_languages();
        assert!(
            !langs.is_empty(),
            "Expected at least one language in languages.json"
        );
        for lang in langs {
            assert!(
                !lang.name.is_empty(),
                "Found language entry with empty name"
            );
            assert!(
                !lang.file_patterns.is_empty(),
                "Language '{}' has no file patterns",
                lang.name
            );
        }
    }
}
