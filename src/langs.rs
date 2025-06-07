use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Deserialize, Serialize)]
pub struct Language {
    pub name: String,
    pub file_patterns: Vec<String>,
    pub line_comments: Option<Vec<String>>,
    pub block_comments: Option<Vec<Vec<String>>>,
}

static LANGUAGES_JSON: &str = include_str!("languages.json");
static LANGUAGES: OnceLock<Vec<Language>> = OnceLock::new();

fn get_languages() -> &'static Vec<Language> {
    LANGUAGES.get_or_init(|| {
        serde_json::from_str(LANGUAGES_JSON).expect("Failed to parse embedded languages.json")
    })
}

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
