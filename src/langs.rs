use crate::utils::pluralize;

include!(concat!(env!("OUT_DIR"), "/languages.rs"));

fn matches_pattern(filename: &str, pattern: &str) -> bool {
	pattern.strip_prefix('*').map_or_else(|| filename == pattern, |suffix| filename.ends_with(suffix))
}

/// Detect the programming language of a file based on its filename
///
/// This function uses a combination of exact filename matching and pattern matching
/// to identify programming languages. It first checks for exact matches, then falls
/// back to pattern matching against file extensions and other patterns.
///
/// # Arguments
///
/// * `filename` - The name of the file (with or without path)
///
/// # Returns
///
/// Returns `Some(language_name)` if a language is detected, `None` otherwise.
///
/// # Examples
///
/// ```
/// use codestats::detect_language;
///
/// assert_eq!(detect_language("main.rs"), Some("Rust"));
/// assert_eq!(detect_language("script.py"), Some("Python"));
/// assert_eq!(detect_language("unknown.xyz"), None);
/// ```
#[must_use]
pub fn detect_language(filename: &str) -> Option<&'static str> {
	if let Some(lang) = PATTERN_MAP.get(filename) {
		return Some(lang.name);
	}
	LANGUAGES
		.iter()
		.find(|language| language.file_patterns.iter().any(|pattern| matches_pattern(filename, pattern)))
		.map(|language| language.name)
}

/// Get detailed language information by name
///
/// Returns the complete language configuration including comment patterns,
/// file patterns, and other metadata for the specified language.
///
/// # Arguments
///
/// * `language_name` - The name of the programming language
///
/// # Returns
///
/// Returns `Some(Language)` if the language is supported, `None` otherwise.
#[must_use]
pub fn get_language_info(language_name: &str) -> Option<&'static Language> {
	LANGUAGE_MAP.get(language_name).copied()
}

/// Print all supported programming languages to stdout
///
/// Displays a complete list of all programming languages that can be detected
/// and analyzed by the library. The output includes the total count and names
/// of all supported languages.
pub fn print_all_languages() {
	println!(
		"Total number of supported programming {}: {}",
		pluralize(LANGUAGES.len() as u64, "language", "languages"),
		LANGUAGES.len()
	);
	for (i, lang) in LANGUAGES.iter().enumerate() {
		if i == LANGUAGES.len() - 1 {
			println!("{}.", lang.name);
		} else {
			println!("{},", lang.name);
		}
	}
}
