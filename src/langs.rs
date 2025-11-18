use crate::utils::pluralize;

include!(concat!(env!("OUT_DIR"), "/languages.rs"));

#[inline]
fn matches_pattern(filename: &str, pattern: &str) -> bool {
	pattern.strip_prefix('*').map_or_else(
		|| filename == pattern || filename.eq_ignore_ascii_case(pattern),
		|suffix| filename.ends_with(suffix) || ends_with_ignore_ascii_case(filename, suffix),
	)
}

#[inline]
fn ends_with_ignore_ascii_case(value: &str, suffix: &str) -> bool {
	if suffix.is_empty() {
		return true;
	}
	let Some(start) = value.len().checked_sub(suffix.len()) else {
		return false;
	};
	value.get(start..).is_some_and(|tail| tail.eq_ignore_ascii_case(suffix))
}

#[inline]
fn get_candidates(filename: &str) -> Vec<&'static Language> {
	if let Some(literal_matches) = PATTERN_MAP.get(filename) {
		return literal_matches.to_vec();
	}
	LANGUAGES
		.iter()
		.filter(|lang| lang.file_patterns.iter().any(|pattern| matches_pattern(filename, pattern)))
		.collect()
}

#[inline]
fn score_language(lang: &Language, content: &str) -> i32 {
	let mut score: i32 = 0;
	if lang.line_comments.is_empty() && lang.keywords.is_empty() {
		return 0;
	}
	for comment in lang.line_comments {
		if content.contains(comment) {
			score = score.saturating_add(50);
		}
	}
	for keyword in lang.keywords {
		let count = content.matches(keyword).count();
		let clamped_count = count.min(usize::try_from(i32::MAX / 10).unwrap_or(usize::MAX));
		// We now know that this is safe because we've clamped the value.
		#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
		let count_i32 = clamped_count as i32;
		score = score.saturating_add(count_i32.saturating_mul(10));
	}
	score
}

#[inline]
fn disambiguate<'a>(candidates: &[&'a Language], content: &str) -> Option<&'a Language> {
	candidates
		.iter()
		.map(|lang| (*lang, score_language(lang, content)))
		.max_by_key(|(_, score)| *score)
		.filter(|(_, score)| *score > 0)
		.map(|(lang, _)| lang)
}

/// Detect the full [`Language`] metadata for a file, optionally using its contents for disambiguation between extensions that map to multiple languages.
///
/// # Examples
/// ```
/// use codestats::langs::detect_language_info;
///
/// let language = detect_language_info("main.rs", None).unwrap();
/// assert_eq!(language.name, "Rust");
/// assert!(language.line_comments.contains(&"//"));
/// ```
#[must_use]
pub fn detect_language_info(filename: &str, content: Option<&str>) -> Option<&'static Language> {
	let candidates = get_candidates(filename);
	match candidates.len() {
		0 => None,
		1 => Some(candidates[0]),
		_ => content
			.and_then(|file_content| disambiguate(&candidates, file_content))
			.or_else(|| candidates.first().copied()),
	}
}

/// Detect only the language name for a file. Prefer [`detect_language_info`] when you need the metadata associated with the language.
///
/// # Examples
/// ```
/// use codestats::langs::detect_language;
///
/// assert_eq!(detect_language("main.rs", None), Some("Rust"));
/// assert_eq!(detect_language("script.py", None), Some("Python"));
/// ```
#[must_use]
pub fn detect_language(filename: &str, content: Option<&str>) -> Option<&'static str> {
	detect_language_info(filename, content).map(|lang| lang.name)
}

/// Look up language metadata by its canonical name.
///
/// # Examples
/// ```
/// use codestats::langs::get_language_info;
///
/// let rust = get_language_info("Rust").unwrap();
/// assert_eq!(rust.name, "Rust");
/// assert!(rust.file_patterns.contains(&"*.rs"));
/// ```
#[must_use]
pub fn get_language_info(language_name: &str) -> Option<&'static Language> {
	LANGUAGE_MAP.get(language_name).copied()
}

pub fn print_all_languages() {
	let lang_count = u64::try_from(LANGUAGES.len()).unwrap_or(u64::MAX);
	println!(
		"Total number of supported programming {}: {}",
		pluralize(lang_count, "language", "languages"),
		LANGUAGES.len()
	);
	let last_idx = LANGUAGES.len().saturating_sub(1);
	for (i, lang) in LANGUAGES.iter().enumerate() {
		let suffix = if i == last_idx { "." } else { "," };
		println!("{}{suffix}", lang.name);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const TEST_LANGUAGE_ALPHA: Language = Language {
		name: "Alpha",
		file_patterns: &["*.alpha"],
		line_comments: &["//"],
		block_comments: &[],
		nested_blocks: false,
		shebangs: &[],
		keywords: &["alpha", "beta"],
	};

	const TEST_LANGUAGE_BETA: Language = Language {
		name: "Beta",
		file_patterns: &["*.beta"],
		line_comments: &["#"],
		block_comments: &[],
		nested_blocks: false,
		shebangs: &[],
		keywords: &["winner"],
	};

	#[test]
	fn matches_pattern_understands_wildcards() {
		assert!(matches_pattern("main.rs", "*.rs"));
		assert!(matches_pattern("README.MD", "*.md"));
		assert!(!matches_pattern("main.rs", "*.py"));
	}

	#[test]
	fn matches_pattern_is_case_insensitive_for_literals() {
		assert!(matches_pattern("MAKEFILE", "Makefile"));
		assert!(matches_pattern("Dockerfile", "dockerfile"));
		assert!(matches_pattern("CMakeLists.txt", "cmakelists.txt"));
		assert!(!matches_pattern("Cargo.toml", "Makefile"));
	}

	#[test]
	fn get_candidates_uses_literal_map() {
		let candidates = get_candidates("Makefile");
		assert_eq!(candidates.len(), 1);
		assert_eq!(candidates[0].name, "Makefile");
	}

	#[test]
	fn get_candidates_handles_case_insensitive_literals() {
		let candidates = get_candidates("MAKEFILE");
		assert_eq!(candidates.len(), 1);
		assert_eq!(candidates[0].name, "Makefile");
	}

	#[test]
	fn get_candidates_supports_wildcards() {
		let candidates = get_candidates("lib.rs");
		assert!(candidates.iter().any(|lang| lang.name == "Rust"));
	}

	#[test]
	fn score_language_combines_comments_and_keywords() {
		let content = "// comment\nalpha beta alpha";
		let score = score_language(&TEST_LANGUAGE_ALPHA, content);
		assert_eq!(score, 50 + 3 * 10);
	}

	#[test]
	fn disambiguate_prefers_highest_score() {
		let candidates = vec![&TEST_LANGUAGE_ALPHA, &TEST_LANGUAGE_BETA];
		let alpha_content = "alpha only";
		let beta_content = "# winner winner";
		let chosen_alpha = disambiguate(&candidates, alpha_content).unwrap();
		assert_eq!(chosen_alpha.name, "Alpha");
		let chosen_beta = disambiguate(&candidates, beta_content).unwrap();
		assert_eq!(chosen_beta.name, "Beta");
	}

	#[test]
	fn detect_language_info_disambiguates_real_languages() {
		let content = "@interface Foo : NSObject\n@end\n";
		let language = detect_language_info("example.m", Some(content)).unwrap();
		assert_eq!(language.name, "Objective-C");
	}

	#[test]
	fn detect_language_info_falls_back_to_first_candidate_without_content_signal() {
		let language = detect_language_info("ambiguous.m", Some("plain text without hints")).unwrap();
		assert_eq!(language.name, "MATLAB");
	}

	#[test]
	fn detect_language_returns_language_name() {
		assert_eq!(detect_language("analysis.ts", None), Some("TypeScript"));
	}

	#[test]
	fn get_language_info_finds_known_language() {
		let language = get_language_info("Rust").expect("language should exist");
		assert_eq!(language.name, "Rust");
		assert!(language.file_patterns.contains(&"*.rs"));
	}
}
