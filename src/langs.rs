use crate::utils::pluralize;

include!(concat!(env!("OUT_DIR"), "/languages.rs"));

fn matches_pattern(filename: &str, pattern: &str) -> bool {
	if let Some(suffix) = pattern.strip_prefix('*') { filename.ends_with(suffix) } else { filename == pattern }
}

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

#[must_use]
pub fn get_language_info(language_name: &str) -> Option<&'static Language> {
	LANGUAGE_MAP.get(language_name).copied()
}

/// Prints all supported language names to stdout
pub fn print_all() {
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
