use crate::utils::pluralize;

include!(concat!(env!("OUT_DIR"), "/languages.rs"));

#[inline]
fn matches_pattern(filename: &str, pattern: &str) -> bool {
	pattern.strip_prefix('*').map_or_else(|| filename == pattern, |suffix| filename.ends_with(suffix))
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

#[must_use]
pub fn detect_language(filename: &str, content: Option<&str>) -> Option<&'static str> {
	detect_language_info(filename, content).map(|lang| lang.name)
}

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
