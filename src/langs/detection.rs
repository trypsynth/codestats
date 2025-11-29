use super::{
	data::{LANGUAGES, Language},
	globset::get_candidates,
};

#[inline]
fn score_language(lang: &Language, content: &str, tokens: &[&str]) -> i32 {
	let mut score: i32 = 0;
	if lang.line_comments.is_empty() && lang.block_comments.is_empty() && lang.keywords.is_empty() {
		return 0;
	}
	for comment in lang.line_comments {
		if content.contains(comment) {
			score = score.saturating_add(50);
		}
	}
	for comment_pair in lang.block_comments {
		if content.contains(comment_pair.0) && content.contains(comment_pair.1) {
			score = score.saturating_add(50);
		}
	}
	for keyword in lang.keywords {
		// If keyword contains special characters, use substring matching to handle cases like "@interface" in Objective-C, which wouldn't match via tokenization since @ is a delimiter.
		let count = if keyword.chars().any(|c| !c.is_ascii_alphanumeric() && c != '_') {
			content.matches(keyword).count()
		} else {
			tokens.iter().filter(|token| token.eq_ignore_ascii_case(keyword)).count()
		};
		let clamped_count = count.min(usize::try_from(i32::MAX / 10).unwrap_or(usize::MAX));
		// We now know that this is safe because we've clamped the value.
		#[expect(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
		let count_i32 = clamped_count as i32;
		score = score.saturating_add(count_i32.saturating_mul(10));
	}
	score
}

#[inline]
fn disambiguate<'a>(candidates: &[&'a Language], content: &str) -> Option<&'a Language> {
	let tokens = tokenize(content);
	candidates
		.iter()
		.map(|lang| (*lang, score_language(lang, content, &tokens)))
		.max_by_key(|(_, score)| *score)
		.filter(|(_, score)| *score > 0)
		.map(|(lang, _)| lang)
}

#[inline]
fn tokenize(content: &str) -> Vec<&str> {
	content.split(|c: char| !c.is_ascii_alphanumeric() && c != '_').filter(|token| !token.is_empty()).collect()
}

#[inline]
fn detect_from_shebang(content: &str) -> Option<&'static Language> {
	let first_line = content.lines().next()?;
	let trimmed = first_line.trim();
	if !trimmed.starts_with("#!") {
		return None;
	}
	LANGUAGES
		.iter()
		.find(|lang| !lang.shebangs.is_empty() && lang.shebangs.iter().any(|shebang| trimmed.starts_with(shebang)))
}

#[must_use]
pub fn detect_language_info(filename: &str, content: Option<&str>) -> Option<&'static Language> {
	let candidates = get_candidates(filename);
	match candidates.len() {
		0 => content.and_then(detect_from_shebang),
		1 => Some(candidates[0]),
		_ => {
			let detected = content.and_then(|file_content| {
				detect_from_shebang(file_content).or_else(|| disambiguate(&candidates, file_content))
			});
			detected.or_else(|| candidates.first().copied())
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	const TEST_LANGUAGE_ALPHA: Language = Language {
		index: 0,
		name: "Alpha",
		file_patterns: &["*.alpha"],
		line_comments: &["//"],
		block_comments: &[],
		nested_blocks: false,
		shebangs: &[],
		keywords: &["alpha", "beta"],
	};

	const TEST_LANGUAGE_BETA: Language = Language {
		index: 1,
		name: "Beta",
		file_patterns: &["*.beta"],
		line_comments: &["#"],
		block_comments: &[],
		nested_blocks: false,
		shebangs: &[],
		keywords: &["winner"],
	};

	#[test]
	fn score_language_combines_comments_and_keywords() {
		let content = "// comment\nalpha beta alpha";
		let tokens = tokenize(content);
		let score = score_language(&TEST_LANGUAGE_ALPHA, content, &tokens);
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
}
