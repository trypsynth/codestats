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
    fn test_detect_language() {
        assert_eq!(detect_language("test.rs"), Some("Rust".to_string()));
        assert_eq!(detect_language("test.py"), Some("Python".to_string()));
        assert_eq!(detect_language("test.unknown"), None);
    }

    #[test]
    fn test_exact_match() {
        assert_eq!(detect_language("Makefile"), Some("Makefile".to_string()));
        assert_eq!(
            detect_language("Dockerfile"),
            Some("Dockerfile".to_string())
        );
    }
}
