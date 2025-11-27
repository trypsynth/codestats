use std::{io::Write, sync::LazyLock};

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use anyhow::Result;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

use crate::utils::pluralize;

include!(concat!(env!("OUT_DIR"), "/languages.rs"));

static LANGUAGE_GLOBSET: LazyLock<GlobSet> = LazyLock::new(|| {
	let mut builder = GlobSetBuilder::new();
	for lang in LANGUAGES {
		for pattern in lang.file_patterns {
			let mut glob_builder = GlobBuilder::new(pattern);
			glob_builder.case_insensitive(true);
			let glob = glob_builder
				.build()
				.unwrap_or_else(|e| panic!("Invalid glob pattern '{pattern}' for language {}: {e}", lang.name));
			builder.add(glob);
		}
	}
	builder.build().expect("Failed to build language globset")
});

#[derive(Debug)]
pub(crate) struct LanguageMatchers {
	pub(crate) line_comments: Option<AhoCorasick>,
	pub(crate) block_comments: Option<BlockCommentMatchers>,
}

#[derive(Debug, Clone, Copy)]
enum BlockPatternKind {
	Start,
	End,
}

#[derive(Debug)]
pub(crate) struct BlockCommentMatchers {
	automaton: AhoCorasick,
	kinds: Vec<BlockPatternKind>,
}

impl BlockCommentMatchers {
	#[inline]
	pub(crate) fn find_block_start(&self, line: &str) -> Option<(usize, usize)> {
		self.automaton.find_iter(line).find_map(|m| {
			if matches!(self.kinds[m.pattern().as_usize()], BlockPatternKind::Start) {
				Some((m.start(), m.len()))
			} else {
				None
			}
		})
	}

	#[inline]
	pub(crate) fn find_block_end_or_nested_start(&self, line: &str, nested: bool) -> Option<(usize, usize, bool)> {
		for m in self.automaton.find_iter(line) {
			match self.kinds[m.pattern().as_usize()] {
				BlockPatternKind::Start if nested => return Some((m.start(), m.len(), true)),
				BlockPatternKind::End => return Some((m.start(), m.len(), false)),
				_ => {}
			}
		}
		None
	}
}

static LANGUAGE_MATCHERS: LazyLock<Vec<LanguageMatchers>> =
	LazyLock::new(|| LANGUAGES.iter().map(build_language_matchers).collect());

#[inline]
pub(crate) fn language_matchers(lang: &Language) -> &'static LanguageMatchers {
	&LANGUAGE_MATCHERS[lang.index]
}

fn build_language_matchers(lang: &Language) -> LanguageMatchers {
	let line_comments = if lang.line_comments.is_empty() {
		None
	} else {
		Some(
			AhoCorasickBuilder::new()
				.match_kind(MatchKind::LeftmostFirst)
				.build(lang.line_comments)
				.expect("Failed to build line comment matcher"),
		)
	};
	let block_comments = if lang.block_comments.is_empty() {
		None
	} else {
		let pattern_capacity = lang.block_comments.len().saturating_mul(2);
		let mut patterns = Vec::with_capacity(pattern_capacity);
		let mut kinds = Vec::with_capacity(pattern_capacity);
		for (start, end) in lang.block_comments {
			patterns.push(*start);
			kinds.push(BlockPatternKind::Start);
			patterns.push(*end);
			kinds.push(BlockPatternKind::End);
		}
		let automaton = AhoCorasickBuilder::new()
			.match_kind(MatchKind::LeftmostFirst)
			.build(patterns)
			.expect("Failed to build block comment matcher");
		Some(BlockCommentMatchers { automaton, kinds })
	};
	LanguageMatchers { line_comments, block_comments }
}

#[inline]
fn matches_pattern(filename: &str, pattern: &str) -> bool {
	pattern.strip_prefix('*').map_or_else(
		|| filename == pattern || filename.eq_ignore_ascii_case(pattern),
		|suffix| filename.ends_with(suffix) || ends_with_ignore_ascii_case(filename, suffix),
	)
}

#[inline]
fn ends_with_ignore_ascii_case(value: &str, suffix: &str) -> bool {
	let value_bytes = value.as_bytes();
	let suffix_bytes = suffix.as_bytes();
	if value_bytes.len() < suffix_bytes.len() {
		return false;
	}
	value_bytes.iter().rev().zip(suffix_bytes.iter().rev()).all(|(a, b)| a.eq_ignore_ascii_case(b))
}

/// Precompiled, case-insensitive globset of all known language file patterns.
#[must_use]
pub(crate) fn language_globset() -> &'static GlobSet {
	&LANGUAGE_GLOBSET
}

#[inline]
pub(crate) fn get_candidates(filename: &str) -> Vec<&'static Language> {
	if let Some(literal_matches) = PATTERN_MAP.get(filename) {
		return literal_matches.to_vec();
	}
	LANGUAGES
		.iter()
		.filter(|lang| lang.file_patterns.iter().any(|pattern| matches_pattern(filename, pattern)))
		.collect()
}

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
		#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
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

/// Detect language from shebang line (e.g., `#!/bin/bash`).
///
/// Returns the first language whose shebang patterns match the beginning of the content.
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

/// Detect the full [`Language`] metadata for a file, optionally using its contents for disambiguation between extensions that map to multiple languages.
///
/// If no filename patterns match, falls back to shebang detection when content is provided.
///
/// # Examples
/// ```
/// use codestats::langs::detect_language_info;
///
/// let language = detect_language_info("main.rs", None).unwrap();
/// assert_eq!(language.name, "Rust");
/// assert!(language.line_comments.contains(&"//"));
/// // Extensionless file with shebang
/// let script = detect_language_info("my-script", Some("#!/bin/bash\necho hello")).unwrap();
/// assert_eq!(script.name, "Bash");
/// ```
#[must_use]
pub fn detect_language_info(filename: &str, content: Option<&str>) -> Option<&'static Language> {
	let candidates = get_candidates(filename);
	match candidates.len() {
		0 => content.and_then(detect_from_shebang),
		1 => Some(candidates[0]),
		_ => content
			.and_then(|file_content| disambiguate(&candidates, file_content))
			.or_else(|| candidates.first().copied()),
	}
}

/// Print all supported programming languages to stdout.
///
/// # Errors
///
/// Returns an error if writing to the provided writer fails.
pub fn print_all_languages(writer: &mut dyn Write) -> Result<()> {
	let lang_count = u64::try_from(LANGUAGES.len()).unwrap_or(u64::MAX);
	writeln!(
		writer,
		"Total number of supported programming {}: {}",
		pluralize(lang_count, "language", "languages"),
		LANGUAGES.len()
	)?;
	let last_idx = LANGUAGES.len().saturating_sub(1);
	for (i, lang) in LANGUAGES.iter().enumerate() {
		let suffix = if i == last_idx { "." } else { "," };
		writeln!(writer, "{}{suffix}", lang.name)?;
	}
	Ok(())
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
	fn globset_matches_common_extensions() {
		assert!(language_globset().is_match("main.rs"));
		assert!(language_globset().is_match("README.md"));
	}

	#[test]
	fn matches_pattern_handles_unicode_filenames() {
		let filename = "report \u{202f}PM.PDF";
		assert!(matches_pattern(filename, "*.pdf"));
	}

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
