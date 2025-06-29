use crate::utils::pluralize;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Language {
	pub name: String,
	pub file_patterns: Vec<String>,
	pub line_comments: Option<Vec<String>>,
	pub block_comments: Option<Vec<Vec<String>>>,
	pub nested_blocks: Option<bool>,
}

static LANGUAGES_JSON: &str = include_str!("languages.json");
static LANGUAGES: OnceLock<Vec<Language>> = OnceLock::new();

/// Returns a static reference to the deserialized language data, as loaded from `languages.json`.
fn get_languages() -> &'static Vec<Language> {
	LANGUAGES.get_or_init(|| serde_json::from_str(LANGUAGES_JSON).expect("Failed to parse embedded languages.json"))
}

fn matches_pattern(filename: &str, pattern: &str) -> bool {
	if let Some(suffix) = pattern.strip_prefix('*') { filename.ends_with(suffix) } else { filename == pattern }
}

#[must_use]
pub fn detect_language(filename: &str) -> Option<String> {
	get_languages()
		.iter()
		.find(|language| language.file_patterns.iter().any(|pattern| matches_pattern(filename, pattern)))
		.map(|language| language.name.clone())
}

#[must_use]
pub fn get_language_info(language_name: &str) -> Option<Language> {
	get_languages().iter().find(|lang| lang.name == language_name).cloned()
}

/// Prints all supported language names to stdout
pub fn print_supported_languages() {
	let langs = get_languages();
	println!(
		"Total number of supported programming {}: {}",
		pluralize(langs.len().try_into().unwrap(), "language", "languages"),
		langs.len()
	);
	for lang in get_languages() {
		println!("{}", lang.name);
	}
}
