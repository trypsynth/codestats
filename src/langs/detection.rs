pub(super) mod patterns;
pub mod scoring;

use std::borrow::Cow;

use self::patterns::get_candidates;
use super::data::{LANGUAGES, Language};

/// Score awarded for each comment style match when disambiguating languages.
const COMMENT_MATCH_SCORE: i32 = 50;
/// Score awarded for each keyword match when disambiguating languages.
const KEYWORD_MATCH_SCORE: i32 = 10;

/// Calculate a language match score based on comment styles and keywords found in content.
///
/// The scoring algorithm works as follows:
/// - Each matching line/block comment pattern adds 50 points
/// - Each keyword occurrence adds 10 points
/// - Symbol-only languages (e.g., Brainfuck) require high symbol density when alphabetic content is present
///
/// This weighted scoring ensures comment patterns (strong indicators) outweigh
/// keywords (weaker indicators that may appear as identifiers in other languages).
#[inline]
fn score_language(lang: &Language, content: &str, tokens: &[&str]) -> i32 {
	if lang.line_comments.is_empty() && lang.block_comments.is_empty() && lang.keywords.is_empty() {
		return 0;
	}
	let mut score: i32 = 0;
	for comment in lang.line_comments {
		if content.contains(comment) {
			score = score.saturating_add(COMMENT_MATCH_SCORE);
		}
	}
	for comment_pair in lang.block_comments {
		if content.contains(comment_pair.0) && content.contains(comment_pair.1) {
			score = score.saturating_add(COMMENT_MATCH_SCORE);
		}
	}
	let mut matched_chars: usize = 0;
	for keyword in lang.keywords {
		// If keyword contains special characters, use substring matching to handle cases like "@interface" in Objective-C, which wouldn't match via tokenization since @ is a delimiter.
		let count = if keyword.chars().any(|c| !c.is_ascii_alphanumeric() && c != '_') {
			let occurrences = content.matches(keyword).count();
			matched_chars = matched_chars.saturating_add(occurrences.saturating_mul(keyword.len()));
			occurrences
		} else {
			tokens.iter().filter(|token| token.eq_ignore_ascii_case(keyword)).count()
		};
		let clamped_count = count.min(usize::try_from(i32::MAX / KEYWORD_MATCH_SCORE).unwrap_or(usize::MAX));
		// We now know that this is safe because we've clamped the value.
		#[expect(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
		let count_i32 = clamped_count as i32;
		score = score.saturating_add(count_i32.saturating_mul(KEYWORD_MATCH_SCORE));
	}
	// For symbol-only languages (all keywords are symbols), require high density if alphabetic content exists
	if is_symbol_only_language(lang) && !tokens.is_empty() {
		// Guard against false positives when a file has a few symbols but mostly alphabetic text.
		let non_whitespace = content.chars().filter(|c| !c.is_whitespace()).count();
		if non_whitespace > 0 {
			let matched_chars_u128 = matched_chars as u128;
			let non_whitespace_u128 = non_whitespace as u128;
			// Require at least 50% of non-whitespace chars to be language symbols
			if matched_chars_u128.saturating_mul(2) < non_whitespace_u128 {
				return 0;
			}
		}
	}
	score
}

fn is_symbol_only_language(lang: &Language) -> bool {
	!lang.keywords.is_empty()
		&& lang.keywords.iter().all(|kw| kw.chars().all(|c| !c.is_ascii_alphanumeric() && c != '_'))
		&& lang.line_comments.is_empty()
		&& lang.block_comments.is_empty()
}

#[inline]
fn disambiguate<'a>(candidates: &[&'a Language], content: &str) -> Option<&'a Language> {
	let tokens: Vec<_> = tokenize(content).collect();
	candidates
		.iter()
		.map(|lang| (*lang, score_language(lang, content, &tokens)))
		.max_by_key(|(_, score)| *score)
		.filter(|(_, score)| *score > 0)
		.map(|(lang, _)| lang)
}

#[inline]
fn tokenize(content: &str) -> impl Iterator<Item = &str> {
	content.split(|c: char| !c.is_ascii_alphanumeric() && c != '_').filter(|token| !token.is_empty())
}

/// Normalize a shebang line by removing optional space after `#!`.
#[inline]
fn normalize_shebang(line: &str) -> std::borrow::Cow<'_, str> {
	line.strip_prefix("#! ").map_or(Cow::Borrowed(line), |rest| Cow::Owned(format!("#!{rest}")))
}

#[inline]
fn detect_from_shebang(content: &str) -> Option<&'static Language> {
	let first_line = content.lines().next()?;
	let trimmed = first_line.trim();
	if !trimmed.starts_with("#!") {
		return None;
	}
	let normalized = normalize_shebang(trimmed);
	LANGUAGES
		.iter()
		.find(|lang| !lang.shebangs.is_empty() && lang.shebangs.iter().any(|shebang| normalized.starts_with(shebang)))
}

#[must_use]
pub fn detect_language_info(filename: &str, content: Option<&str>) -> Option<&'static Language> {
	let candidates = get_candidates(filename);
	match candidates.len() {
		0 => content.and_then(detect_from_shebang),
		1 => Some(candidates[0]),
		_ => content.and_then(|file_content| {
			detect_from_shebang(file_content).or_else(|| disambiguate(&candidates, file_content))
		}),
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
		let tokens: Vec<_> = tokenize(content).collect();
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
	fn detect_language_info_skips_when_no_signal() {
		let language = detect_language_info("ambiguous.m", Some("plain text without hints"));
		assert!(language.is_none());
	}

	#[test]
	fn detect_brainfuck_with_ascii_comments() {
		let content =
			"This is a comment\n++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.\nMore comments here\n>+++.\n";
		let language = detect_language_info("example.bf", Some(content)).unwrap();
		assert_eq!(language.name, "Brainfuck");
	}

	#[test]
	fn detect_b_over_brainfuck() {
		let content = "/* B language */\nmain $(\nauto i;\ni = 0;\nwhile (i < 10) i++;$)";
		let language = detect_language_info("example.b", Some(content)).unwrap();
		assert_eq!(language.name, "B");
	}

	#[test]
	fn detect_b_with_many_comparison_operators() {
		let content = "main $(\n   auto ch;\n   if (ch > 0100 & ch < 0133) ch = ch + 040;\n   if (ch > 500 & ch < 600) goto loop;\n$)";
		let language = detect_language_info("example.b", Some(content)).unwrap();
		assert_eq!(language.name, "B");
	}

	#[test]
	fn tokenize_splits_on_non_alphanumeric() {
		let tokens: Vec<_> = tokenize("hello_world foo-bar").collect();
		assert_eq!(tokens, vec!["hello_world", "foo", "bar"]);
	}

	#[test]
	fn tokenize_handles_empty_string() {
		let tokens: Vec<_> = tokenize("").collect();
		assert!(tokens.is_empty());
	}

	#[test]
	fn tokenize_handles_only_delimiters() {
		let tokens: Vec<_> = tokenize("!@#$%^&*()").collect();
		assert!(tokens.is_empty());
	}

	#[test]
	fn tokenize_preserves_underscores() {
		let tokens: Vec<_> = tokenize("__init__ __main__").collect();
		assert_eq!(tokens, vec!["__init__", "__main__"]);
	}

	#[test]
	fn normalize_shebang_removes_space() {
		let result = normalize_shebang("#! /usr/bin/env python");
		assert_eq!(&*result, "#!/usr/bin/env python");
	}

	#[test]
	fn normalize_shebang_leaves_compact_unchanged() {
		let result = normalize_shebang("#!/bin/bash");
		assert_eq!(&*result, "#!/bin/bash");
	}

	#[test]
	fn detect_from_shebang_finds_python() {
		let content = "#!/usr/bin/env python3\nprint('hello')";
		let lang = detect_from_shebang(content);
		assert!(lang.is_some());
		assert_eq!(lang.unwrap().name, "Python");
	}

	#[test]
	fn detect_from_shebang_finds_bash() {
		let content = "#!/bin/bash\necho hello";
		let lang = detect_from_shebang(content);
		assert!(lang.is_some());
		assert_eq!(lang.unwrap().name, "Bash");
	}

	#[test]
	fn detect_from_shebang_returns_none_without_shebang() {
		let content = "print('hello')\n# not a shebang";
		assert!(detect_from_shebang(content).is_none());
	}

	#[test]
	fn detect_from_shebang_handles_empty() {
		assert!(detect_from_shebang("").is_none());
	}

	#[test]
	fn is_symbol_only_language_detects_brainfuck() {
		// Find Brainfuck in LANGUAGES
		let bf = LANGUAGES.iter().find(|l| l.name == "Brainfuck");
		assert!(bf.is_some());
		assert!(is_symbol_only_language(bf.unwrap()));
	}

	#[test]
	fn is_symbol_only_language_rejects_rust() {
		let rust = LANGUAGES.iter().find(|l| l.name == "Rust");
		assert!(rust.is_some());
		assert!(!is_symbol_only_language(rust.unwrap()));
	}

	#[test]
	fn single_candidate_returns_immediately() {
		// .rs files should have only Rust as candidate
		let lang = detect_language_info("test.rs", None);
		assert!(lang.is_some());
		assert_eq!(lang.unwrap().name, "Rust");
	}

	#[test]
	fn unknown_extension_with_shebang() {
		let content = "#!/usr/bin/env node\nconsole.log('hello');";
		let lang = detect_language_info("script", Some(content));
		assert!(lang.is_some());
		assert_eq!(lang.unwrap().name, "JavaScript");
	}

	#[test]
	fn unknown_extension_without_hints_returns_none() {
		let lang = detect_language_info("random_file", Some("just some text"));
		assert!(lang.is_none());
	}
}
