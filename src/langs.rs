use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Represents a single programming language.
#[derive(Debug, Deserialize, Serialize, Clone)]
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
	LANGUAGES.get_or_init(|| serde_json::from_str(LANGUAGES_JSON).expect("Failed to parse embedded languages.json"))
}

/// Checks if a filename matches a given pattern.
fn matches_pattern(filename: &str, pattern: &str) -> bool {
	if let Some(suffix) = pattern.strip_prefix('*') {
		filename.ends_with(suffix)
	} else {
		filename == pattern
	}
}

/// Tries to detect a programming language given a filename.
#[must_use]
pub fn detect_language(filename: &str) -> Option<String> {
	get_languages()
		.iter()
		.find(|language| language.file_patterns.iter().any(|pattern| matches_pattern(filename, pattern)))
		.map(|language| language.name.clone())
}

/// Returns the language information for a given language name.
#[must_use]
pub fn get_language_info(language_name: &str) -> Option<Language> {
	get_languages().iter().find(|lang| lang.name == language_name).cloned()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_detect_language_known_extensions() {
		let test_cases = [
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
		for (filename, expected) in test_cases {
			let detected = detect_language(filename);
			assert_eq!(detected.as_deref(), expected, "Expected {expected:?} for file {filename:?}, got {detected:?}");
		}
	}

	#[test]
	fn test_detect_language_exact_match_special_filenames() {
		let test_cases = [
			("Makefile", Some("Makefile")),
			("Dockerfile", Some("Dockerfile")),
			("Rakefile", Some("Ruby")),
			("build.gradle", Some("Gradle")),
			("CMakeLists.txt", Some("CMake")),
		];
		for (filename, expected) in test_cases {
			let detected = detect_language(filename);
			assert_eq!(
				detected.as_deref(),
				expected,
				"Expected {expected:?} for special file {filename:?}, got {detected:?}"
			);
		}
	}

	#[test]
	fn test_detect_language_case_sensitivity() {
		let test_cases = [("makefile", Some("Makefile")), ("MAKEFILE", None), ("Dockerfile", Some("Dockerfile"))];
		for (filename, expected) in test_cases {
			let detected = detect_language(filename);
			assert_eq!(
				detected.as_deref(),
				expected,
				"Expected {expected:?} for case test file {filename:?}, got {detected:?}"
			);
		}
	}

	#[test]
	fn test_detect_language_edge_cases() {
		let test_cases = [("", None), (".hidden", None), ("weird.file.name.rs", Some("Rust")), ("tricky.rs.txt", None)];
		for (filename, expected) in test_cases {
			let detected = detect_language(filename);
			assert_eq!(
				detected.as_deref(),
				expected,
				"Expected {expected:?} for edge case file {filename:?}, got {detected:?}"
			);
		}
	}

	#[test]
	fn test_languages_json_is_valid() {
		let langs = get_languages();
		assert!(!langs.is_empty(), "Expected at least one language in languages.json");
		for lang in langs {
			assert!(!lang.name.is_empty(), "Found language entry with empty name");
			assert!(!lang.file_patterns.is_empty(), "Language '{}' has no file patterns", lang.name);
		}
	}

	#[test]
	fn test_get_language_info() {
		let rust_info = get_language_info("Rust");
		assert!(rust_info.is_some());
		let rust = rust_info.unwrap();
		assert_eq!(rust.name, "Rust");
		assert!(rust.line_comments.is_some());
		assert!(rust.block_comments.is_some());
		let unknown_info = get_language_info("UnknownLanguage");
		assert!(unknown_info.is_none());
	}

	#[test]
	fn test_language_comment_info() {
		let rust_info = get_language_info("Rust").unwrap();
		if let Some(ref line_comments) = rust_info.line_comments {
			assert!(line_comments.contains(&"//".to_string()));
		}
		if let Some(ref block_comments) = rust_info.block_comments {
			let found_block =
				block_comments.iter().any(|block| block.len() >= 2 && block[0] == "/*" && block[1] == "*/");
			assert!(found_block, "Expected to find /* */ block comments for Rust");
		}
	}
}
