pub(super) mod patterns;
pub mod scoring;

use std::{
	borrow::Cow,
	sync::{LazyLock, OnceLock},
};

use memchr::memmem::Finder;

pub use self::patterns::get_candidates;
use super::data::{LANGUAGES, Language};

/// Score awarded for each comment style match when disambiguating languages.
const COMMENT_MATCH_SCORE: i32 = 50;
/// Score awarded for each keyword match when disambiguating languages.
const KEYWORD_MATCH_SCORE: i32 = 10;
/// Occurrences of a single keyword that count toward the score, so one identifier repeated hundreds of times (`mapping` in a C file, scored for LPC) can't outweigh several distinct keywords.
const MAX_KEYWORD_OCCURRENCES: usize = 3;

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
/// `candidates` are all the languages being weighed, `lang` among them: a keyword every other one shares is no evidence for `lang`. Comment markers are looked for in `content`, keywords in `code`, which may have comments removed so their prose doesn't count as code ("virtual memory" as C++).
fn score_language(lang: &Language, candidates: &[&Language], content: &str, code: &str, tokens: &Tokens) -> i32 {
	if lang.line_comments.is_empty() && lang.block_comments.is_empty() && lang.keywords.is_empty() {
		return 0;
	}
	let matchers = scoring_matchers(lang);
	let bytes = content.as_bytes();
	let mut score: i32 = 0;
	for comment in lang.line_comments {
		if has_line_comment(content, comment) {
			score = score.saturating_add(COMMENT_MATCH_SCORE);
		}
	}
	for (start, end) in &matchers.block_comments {
		if start.find(bytes).is_some() && end.find(bytes).is_some() {
			score = score.saturating_add(COMMENT_MATCH_SCORE);
		}
	}
	let mut counts = vec![0usize; lang.keywords.len()];
	if !matchers.words.is_empty() {
		for token in &tokens.words {
			if !matchers.word_starts[usize::from(token[0])] {
				continue;
			}
			for &(keyword, index) in &matchers.words {
				let is_match =
					if matchers.case_sensitive { *token == keyword } else { token.eq_ignore_ascii_case(keyword) };
				if is_match {
					counts[index] += 1;
				}
			}
		}
	}
	// Only the capped count matters, so stop counting early; symbol-only languages need every match for their density check.
	let limit = if matchers.symbol_only { usize::MAX } else { MAX_KEYWORD_OCCURRENCES };
	let mut matched_chars: usize = 0;
	for substring in &matchers.substrings {
		let occurrences = substring.count(code.as_bytes(), limit);
		matched_chars = matched_chars.saturating_add(occurrences.saturating_mul(substring.finder.needle().len()));
		counts[substring.index] = occurrences;
	}
	// Whether a keyword tells `lang` apart from the other candidates, rather than being one they all have (`#include` for C and C++).
	let mut others = candidates.iter().filter(|other| !std::ptr::eq(**other, lang)).peekable();
	let has_others = others.peek().is_some();
	let distinctive = |keyword: &str| !has_others || others.clone().any(|other| !other.keywords.contains(&keyword));
	let mut distinctive_hits: usize = 0;
	for (index, count) in counts.into_iter().enumerate() {
		let count = count.min(limit);
		if count > 0 && distinctive(lang.keywords[index]) {
			distinctive_hits = distinctive_hits.saturating_add(count);
		}
		let capped = i32::try_from(count.min(MAX_KEYWORD_OCCURRENCES)).unwrap_or(0);
		score = score.saturating_add(capped * KEYWORD_MATCH_SCORE);
	}
	// Languages that share junk-drawer extensions (e.g. `.inc`) need real keyword evidence, not just a matching comment marker or keywords every candidate has.
	if lang.require_keywords && distinctive_hits == 0 {
		return 0;
	}
	// For symbol-only languages (all keywords are symbols), require high density if alphabetic content exists
	if matchers.symbol_only && tokens.any {
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

/// What scoring looks for in a sample, prepared the first time a language is a candidate.
struct ScoringMatchers {
	/// Keywords made of identifier characters, matched against whole tokens ignoring case, with their index in the language's keywords.
	words: Vec<(&'static [u8], usize)>,
	/// Which bytes (either case) start a word keyword, so most tokens are ruled out with one lookup.
	word_starts: [bool; 256],
	/// The longest word keyword's length.
	max_word_len: usize,
	case_sensitive: bool,
	/// Keywords containing other characters (`@interface`, `#include`), which tokenizing would split, so they're matched as substrings.
	substrings: Vec<SubstringKeyword>,
	block_comments: Vec<(Finder<'static>, Finder<'static>)>,
	symbol_only: bool,
}

struct SubstringKeyword {
	finder: Finder<'static>,
	index: usize,
	/// A keyword ending in an identifier character must not be followed by one, so `@end` doesn't match inside Doxygen's `@endcode`.
	ends_in_word: bool,
}

impl SubstringKeyword {
	/// Count occurrences in `content`, up to `limit`.
	fn count(&self, content: &[u8], limit: usize) -> usize {
		let len = self.finder.needle().len();
		self.finder
			.find_iter(content)
			.filter(|&pos| !self.ends_in_word || content.get(pos + len).is_none_or(|&b| !is_word_byte(b)))
			.take(limit)
			.count()
	}
}

static SCORING_MATCHERS: LazyLock<Vec<OnceLock<ScoringMatchers>>> =
	LazyLock::new(|| LANGUAGES.iter().map(|_| OnceLock::new()).collect());

fn scoring_matchers(lang: &Language) -> &'static ScoringMatchers {
	// Tests score made-up languages that reuse real indices, so they can't share the cache.
	#[cfg(test)]
	if LANGUAGES.get(lang.index).is_none_or(|real| !std::ptr::eq(lang, real)) {
		return Box::leak(Box::new(build_scoring_matchers(lang)));
	}
	SCORING_MATCHERS[lang.index].get_or_init(|| build_scoring_matchers(lang))
}

fn build_scoring_matchers(lang: &Language) -> ScoringMatchers {
	let mut words = Vec::new();
	let mut word_starts = [false; 256];
	let mut substrings = Vec::new();
	for (index, &keyword) in lang.keywords.iter().enumerate() {
		let bytes = keyword.as_bytes();
		if !bytes.is_empty() && bytes.iter().all(|&b| is_word_byte(b)) {
			if lang.case_sensitive_keywords {
				word_starts[usize::from(bytes[0])] = true;
			} else {
				word_starts[usize::from(bytes[0].to_ascii_lowercase())] = true;
				word_starts[usize::from(bytes[0].to_ascii_uppercase())] = true;
			}
			words.push((bytes, index));
		} else {
			let ends_in_word = bytes.last().is_some_and(|&b| is_word_byte(b));
			substrings.push(SubstringKeyword { finder: Finder::new(keyword), index, ends_in_word });
		}
	}
	let block_comments =
		lang.block_comments.iter().map(|&(start, end, _)| (Finder::new(start), Finder::new(end))).collect();
	let max_word_len = words.iter().map(|(word, _)| word.len()).max().unwrap_or(0);
	ScoringMatchers {
		words,
		word_starts,
		max_word_len,
		case_sensitive: lang.case_sensitive_keywords,
		substrings,
		block_comments,
		symbol_only: is_symbol_only_language(lang),
	}
}

const fn is_word_byte(b: u8) -> bool {
	b.is_ascii_alphanumeric() || b == b'_'
}

/// Check whether any line starts with `marker` as a comment.
///
/// The marker must begin the line (after indentation) and must not be immediately followed by an identifier character, so that preprocessor directives like `#import` or format specifiers like `%ld` aren't mistaken for `#` or `%` comments.
fn has_line_comment(content: &str, marker: &str) -> bool {
	content.lines().any(|line| {
		line.trim_start()
			.strip_prefix(marker)
			.is_some_and(|rest| !rest.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_'))
	})
}

fn is_symbol_only_language(lang: &Language) -> bool {
	!lang.keywords.is_empty()
		&& lang.keywords.iter().all(|kw| kw.chars().all(|c| !c.is_ascii_alphanumeric() && c != '_'))
		&& lang.line_comments.is_empty()
		&& lang.block_comments.is_empty()
}

#[inline]
fn same_comments(a: &Language, b: &Language) -> bool {
	a.line_comments == b.line_comments && a.block_comments == b.block_comments
}

/// `content` with `lang`'s comments replaced by spaces, so their prose isn't counted as keywords. String literals aren't recognized, so a marker inside one (`"http://..."`) cuts the line short, which costs a few keywords at most.
fn strip_comments<'a>(content: &'a str, lang: &Language) -> Cow<'a, str> {
	let matchers = scoring::language_matchers(lang);
	if matchers.line_comments.is_none() && matchers.block_comments.is_none() {
		return Cow::Borrowed(content);
	}
	let bytes = content.as_bytes();
	let mut code = String::with_capacity(content.len());
	let mut pos = 0;
	while pos < content.len() {
		let rest = &content[pos..];
		let line = matchers.line_comments.as_ref().and_then(|line_comments| {
			line_comments.find_iter(rest).map(|m| (m.start(), m.end())).find(|&(start, end)| {
				// A marker spelled like a word (`REM`) only counts as a whole word.
				let word_like = rest.as_bytes()[start..end].iter().all(|&b| is_word_byte(b));
				!word_like
					|| (!(pos + start).checked_sub(1).is_some_and(|i| is_word_byte(bytes[i]))
						&& !bytes.get(pos + end).is_some_and(|&b| is_word_byte(b)))
			})
		});
		let block = matchers.block_comments.as_ref().and_then(|block| block.find_block_start(rest));
		let (start, end) = match (line, block) {
			(Some((line_start, _)), Some((block_start, _, _))) if block_start < line_start => {
				block_end(rest, block, matchers)
			}
			(Some((line_start, _)), _) => (
				line_start,
				memchr::memchr(b'\n', &rest.as_bytes()[line_start..]).map_or(rest.len(), |i| line_start + i),
			),
			(None, Some(_)) => block_end(rest, block, matchers),
			(None, None) => {
				code.push_str(rest);
				break;
			}
		};
		code.push_str(&rest[..start]);
		code.push(' ');
		pos += end;
	}
	Cow::Owned(code)
}

/// The start of the block comment found in `rest`, and the end of the comment (after its closing marker, or the end of `rest`), following nesting where the pair nests.
fn block_end(rest: &str, block: Option<(usize, usize, usize)>, matchers: &scoring::LanguageMatchers) -> (usize, usize) {
	let (start, len, pair) = block.expect("called with a block comment start");
	let finder = matchers.block_comments.as_ref().expect("a block comment start was found");
	let mut pos = start + len;
	let mut depth = 1;
	while depth > 0 {
		let Some((found, len, nested)) = finder.find_block_end_or_nested_start(&rest[pos..], pair) else {
			return (start, rest.len());
		};
		depth = if nested { depth + 1 } else { depth - 1 };
		pos += found + len;
	}
	(start, pos)
}

/// Order for breaking ties between equal scores: lower ranks first.
const fn tie_rank(lang: &Language) -> (bool, usize) {
	(lang.require_keywords, lang.keywords.len())
}

/// Pick the best-scoring candidate, returning it with how far it beat the runner-up.
fn disambiguate<'a>(candidates: &[&'a Language], content: &str) -> (Option<&'a Language>, i32) {
	// Only words that could be some candidate's keyword are kept.
	let mut starts = [false; 256];
	let mut max_len = 0;
	for lang in candidates {
		let matchers = scoring_matchers(lang);
		for (start, &can_start) in starts.iter_mut().zip(&matchers.word_starts) {
			*start |= can_start;
		}
		max_len = max_len.max(matchers.max_word_len);
	}
	// When every candidate writes comments the same way (C, C++ and Objective-C for a `.h`), what's in them is prose for all of them, so keywords are only looked for outside them. Otherwise one candidate's comment is another's code, so the whole sample counts.
	let code = match candidates {
		[first, rest @ ..] if rest.iter().all(|other| same_comments(first, other)) => strip_comments(content, first),
		_ => Cow::Borrowed(content),
	};
	let tokens = tokenize(&code, &starts, max_len);
	let mut runner_up = 0;
	// On ties, prefer a candidate that doesn't need keyword evidence (C over C++ for a `.h`), then the one with fewer keywords, then the earliest. A language that extends another (e.g. Bison over Yacc) lists the base language's keywords plus its own, so without extension-specific keywords the base language is the better guess.
	let mut best: Option<(&'a Language, i32)> = None;
	for lang in candidates {
		let score = score_language(lang, candidates, content, &code, &tokens);
		let beats_best = best.is_none_or(|(best_lang, best_score)| {
			score > best_score || (score == best_score && tie_rank(lang) < tie_rank(best_lang))
		});
		if score > 0 && beats_best {
			runner_up = runner_up.max(best.map_or(0, |(_, best_score)| best_score));
			best = Some((lang, score));
		} else {
			runner_up = runner_up.max(score);
		}
	}
	best.map_or((None, 0), |(lang, score)| (Some(lang), score - runner_up))
}

/// The identifier-like words of a sample that could be keywords, and whether it had any words at all.
struct Tokens<'a> {
	words: Vec<&'a [u8]>,
	any: bool,
}

/// Split `content` into identifier-like words (runs of ASCII letters, digits and underscores), keeping only those that begin with a byte in `starts` and are at most `max_len` long.
///
/// Works 64 bytes at a time: each block becomes a mask with a bit per word byte, and words begin and end wherever a bit differs from the one before it, so finding the next is a `trailing_zeros` rather than a branch per byte.
fn tokenize<'a>(content: &'a str, starts: &[bool; 256], max_len: usize) -> Tokens<'a> {
	let bytes = content.as_bytes();
	let mut tokens = Tokens { words: Vec::new(), any: false };
	let mut keep = |start: usize, end: usize| {
		tokens.any = true;
		if end - start <= max_len && starts[usize::from(bytes[start])] {
			tokens.words.push(&bytes[start..end]);
		}
	};
	// Where the word running into the current block began, if one is.
	let mut word_start: Option<usize> = None;
	for (block_index, block) in bytes.chunks(64).enumerate() {
		let base = block_index * 64;
		let mask = word_mask(block);
		// Bits past the end of a short last block are clear, so a word running to the end of `content` ends there too.
		let mut changes = mask ^ ((mask << 1) | u64::from(word_start.is_some()));
		while changes != 0 {
			let pos = base + changes.trailing_zeros() as usize;
			match word_start.take() {
				Some(start) => keep(start, pos),
				None => word_start = Some(pos),
			}
			changes &= changes - 1;
		}
	}
	if let Some(start) = word_start {
		keep(start, bytes.len());
	}
	tokens
}

/// A mask of `block`'s word bytes, bit `i` for byte `i`.
#[inline]
fn word_mask(block: &[u8]) -> u64 {
	let mut mask = 0;
	for (i, chunk) in block.chunks(8).enumerate() {
		let mut word = [0u8; 8];
		word[..chunk.len()].copy_from_slice(chunk);
		mask |= word_bits(u64::from_le_bytes(word)) << (i * 8);
	}
	mask
}

/// For eight bytes packed little-endian into `x`, a bit per word byte (ASCII letter, digit or `_`), found with arithmetic on all eight at once rather than a branch each.
#[inline]
const fn word_bits(x: u64) -> u64 {
	const ONES: u64 = 0x0101_0101_0101_0101;
	const HIGH: u64 = 0x8080_8080_8080_8080;
	// Each byte's high bit is set where `low <= byte < high`, for bytes under 0x80; `x & !HIGH` keeps the sums from carrying between bytes.
	const fn in_range(seven: u64, low: u64, high: u64) -> u64 {
		let at_least_low = seven + (128 - low) * ONES;
		let below_high = seven + (128 - high) * ONES;
		at_least_low & !below_high & HIGH
	}
	let seven = x & !HIGH;
	let folded = (x | 0x2020_2020_2020_2020) & !HIGH;
	let digits = in_range(seven, b'0' as u64, b'9' as u64 + 1);
	let letters = in_range(folded, b'a' as u64, b'z' as u64 + 1);
	let underscore = !((seven ^ (b'_' as u64 * ONES)) + !HIGH) & HIGH;
	// Bytes of 0x80 and above (UTF-8) are never word bytes.
	let set = (digits | letters | underscore) & !x & HIGH;
	// Gather the eight high bits into the low byte.
	(set >> 7).wrapping_mul(0x0102_0408_1020_4080) >> 56
}

/// Normalize a shebang line: drop an optional space after `#!` and env's `-S` flag (`#!/usr/bin/env -S python3 -u`).
#[inline]
fn normalize_shebang(line: &str) -> Cow<'_, str> {
	let mut normalized = line.strip_prefix("#! ").map_or(Cow::Borrowed(line), |rest| Cow::Owned(format!("#!{rest}")));
	if let Some(idx) = normalized.find("/env -S ") {
		normalized = Cow::Owned(format!("{}/env {}", &normalized[..idx], &normalized[idx + "/env -S ".len()..]));
	}
	normalized
}

/// Length of the longest of `lang`'s shebangs that a normalized shebang line starts with. The interpreter name must end there, apart from a version (`python3.12`), so `node` doesn't claim `nodemon`.
#[inline]
fn shebang_match_len(normalized: &str, lang: &Language) -> Option<usize> {
	lang.shebangs
		.iter()
		.filter(|shebang| {
			normalized.strip_prefix(**shebang).is_some_and(|rest| !rest.starts_with(|c: char| c.is_ascii_alphabetic()))
		})
		.map(|shebang| shebang.len())
		.max()
}

/// Whether a (trimmed) first line is a shebang for `lang`.
#[must_use]
#[inline]
pub fn is_shebang_for(line: &str, lang: &Language) -> bool {
	!lang.shebangs.is_empty() && line.starts_with("#!") && shebang_match_len(&normalize_shebang(line), lang).is_some()
}

/// Detect a language from a shebang, preferring the longest match (`perl6` is Raku, not Perl).
#[inline]
fn detect_from_shebang(content: &str) -> Option<&'static Language> {
	let trimmed = content.lines().next()?.trim();
	if !trimmed.starts_with("#!") {
		return None;
	}
	let normalized = normalize_shebang(trimmed);
	// Iterating in reverse makes `max_by_key` keep the earliest language on a tie.
	LANGUAGES
		.iter()
		.rev()
		.filter_map(|lang| shebang_match_len(&normalized, lang).map(|len| (len, lang)))
		.max_by_key(|&(len, _)| len)
		.map(|(_, lang)| lang)
}

/// When nothing scores, fall back to the one candidate that doesn't require keyword evidence, if there is exactly one (e.g. C over Objective-C for a bare `.h`).
fn default_candidate(candidates: &[&'static Language]) -> Option<&'static Language> {
	let mut defaults = candidates.iter().filter(|lang| !lang.require_keywords);
	match (defaults.next(), defaults.next()) {
		(Some(lang), None) => Some(lang),
		_ => None,
	}
}

/// The language detected for a file, and whether content decided it by a narrow margin for an unexpected language, so that more of the file might change the answer.
#[derive(Debug, Clone, Copy)]
pub struct Detection {
	pub language: Option<&'static Language>,
	pub close_call: bool,
}

/// How far the winning candidate must beat the runner-up (three keyword hits) for a decision from a small sample to stand.
const DECISIVE_MARGIN: i32 = 3 * KEYWORD_MATCH_SCORE;

/// Pick a language from a filename's candidates (see [`get_candidates`]). Content is only consulted when there isn't exactly one candidate.
#[must_use]
pub fn detect_language(candidates: &[&'static Language], content: Option<&str>) -> Detection {
	let decided = |language| Detection { language, close_call: false };
	match candidates.len() {
		0 => decided(content.and_then(detect_from_shebang)),
		1 => decided(Some(candidates[0])),
		_ => {
			let Some(file_content) = content else {
				return decided(default_candidate(candidates));
			};
			if let Some(language) = detect_from_shebang(file_content) {
				return decided(Some(language));
			}
			let (language, margin) = disambiguate(candidates, file_content);
			let default = default_candidate(candidates);
			// Only a narrow win for something other than the default (LPC or Objective-C over C) is worth a closer look; a narrow win for the default is the expected answer.
			let surprising = language.is_none() || language.map(|l| l.index) != default.map(|l| l.index);
			Detection { language: language.or(default), close_call: margin < DECISIVE_MARGIN && surprising }
		}
	}
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;

	fn detect_language_info(filename: &str, content: Option<&str>) -> Option<&'static Language> {
		detect_language(&get_candidates(filename), content).language
	}

	const TEST_LANGUAGE_ALPHA: Language = Language {
		index: 0,
		name: "Alpha",
		file_patterns: &["*.alpha"],
		line_comments: &["//"],
		block_comments: &[],
		column_one_comments: &[],
		shebangs: &[],
		keywords: &["alpha", "beta"],
		require_keywords: false,
		case_sensitive_keywords: false,
		c_preprocessor: false,
		regex_literals: false,
		strings: &[],
	};

	const TEST_LANGUAGE_BETA: Language = Language {
		index: 1,
		name: "Beta",
		file_patterns: &["*.beta"],
		line_comments: &["#"],
		block_comments: &[],
		column_one_comments: &[],
		shebangs: &[],
		keywords: &["winner"],
		require_keywords: false,
		case_sensitive_keywords: false,
		c_preprocessor: false,
		regex_literals: false,
		strings: &[],
	};

	#[test]
	fn score_language_combines_comments_and_keywords() {
		let content = "// comment\nalpha beta alpha";
		let score =
			score_language(&TEST_LANGUAGE_ALPHA, &[], content, content, &tokenize(content, &[true; 256], usize::MAX));
		assert_eq!(score, 50 + 3 * 10);
	}

	#[test]
	fn repeated_keyword_is_capped() {
		// One keyword repeated many times shouldn't beat several distinct keywords.
		const REPEATED: Language = Language { name: "Repeated", keywords: &["mapping"], ..TEST_LANGUAGE_ALPHA };
		const DISTINCT: Language =
			Language { name: "Distinct", keywords: &["void", "struct", "sizeof", "typedef"], ..TEST_LANGUAGE_ALPHA };
		let content = format!("// x\n{}void f(struct s); sizeof(int); typedef int t;", "mapping ".repeat(50));
		assert_eq!(disambiguate(&[&REPEATED, &DISTINCT], &content).0.unwrap().name, "Distinct");
	}

	#[test]
	fn disambiguate_prefers_highest_score() {
		let candidates = vec![&TEST_LANGUAGE_ALPHA, &TEST_LANGUAGE_BETA];
		let alpha_content = "alpha only";
		let beta_content = "# winner winner";
		let chosen_alpha = disambiguate(&candidates, alpha_content).0.unwrap();
		assert_eq!(chosen_alpha.name, "Alpha");
		let chosen_beta = disambiguate(&candidates, beta_content).0.unwrap();
		assert_eq!(chosen_beta.name, "Beta");
	}

	#[test]
	fn disambiguate_prefers_base_language_on_tie() {
		const BASE: Language = Language { name: "Base", keywords: &["alpha"], ..TEST_LANGUAGE_ALPHA };
		const EXTENDED: Language = Language { name: "Extended", keywords: &["alpha", "omega"], ..TEST_LANGUAGE_ALPHA };
		let candidates = vec![&EXTENDED, &BASE];
		assert_eq!(disambiguate(&candidates, "// x\nalpha").0.unwrap().name, "Base");
		assert_eq!(disambiguate(&candidates, "// x\nalpha omega").0.unwrap().name, "Extended");
	}

	#[test]
	fn detect_language_info_disambiguates_real_languages() {
		let content = "@interface Foo : NSObject\n@end\n";
		let language = detect_language_info("example.m", Some(content)).unwrap();
		assert_eq!(language.name, "Objective-C");
	}

	#[rstest]
	#[case::directives_and_format_specifiers(
		"example.m",
		"#import <Foundation/Foundation.h>\n#include <stdio.h>\n\nvoid f(NSInteger n) {\n\tNSLog(@\"%ld\", (long)n);\n}\n",
		"Objective-C"
	)]
	#[case::autoreleasepool(
		"main.m",
		"#import \"App.h\"\n\nint main(void) {\n\t@autoreleasepool {\n\t\t[[App alloc] init];\n\t}\n\treturn 0;\n}\n",
		"Objective-C"
	)]
	#[case::objc_header(
		"App.h",
		"#import <Foundation/Foundation.h>\n\n@interface App : NSObject\n- (void)run;\n@end\n",
		"Objective-C"
	)]
	#[case::c_header("util.h", "#ifndef UTIL_H\n#define UTIL_H\n\nint add(int a, int b);\n\n#endif\n", "C")]
	#[case::cpp_header(
		"widget.h",
		"#ifndef WIDGET_H\n#define WIDGET_H\n\nnamespace ui {\nclass Widget : public Base {\npublic:\n\tvirtual ~Widget();\n};\n}\n\n#endif\n",
		"C++"
	)]
	#[case::c_header_mentioning_cpp_in_comments(
		"mm.h",
		"/* Maps virtual memory into the caller's namespace; see the template above. */\n#ifndef MM_H\n#define MM_H\nstruct page;\nvoid map(struct page *p);\n#endif\n",
		"C"
	)]
	#[case::uppercase_is_not_a_cpp_keyword(
		"cfg.h",
		"#define VIRTUAL 1\n#define TEMPLATE 2\ntypedef int OPERATOR;\n",
		"C"
	)]
	#[case::bare_header_defaults_to_c("empty.h", "#pragma once\n", "C")]
	#[case::c_header_comments_only("util.h", "// helpers\n/* nothing here */\n", "C")]
	#[case::octave_function("example.m", "# compute\nfunction y = f(x)\n\ty = x;\nendfunction\n", "Octave")]
	fn detect_m_and_h_files(#[case] filename: &str, #[case] content: &str, #[case] expected: &str) {
		assert_eq!(detect_language_info(filename, Some(content)).map(|l| l.name), Some(expected));
	}

	#[rstest]
	#[case::whole_keyword("@end\n", "@end", 1)]
	#[case::doxygen_command("/** @endcode */", "@end", 0)]
	#[case::followed_by_punctuation("@end;", "@end", 1)]
	#[case::symbol_keyword("a -> b -> c", "->", 2)]
	fn test_count_substring_keyword(#[case] content: &str, #[case] keyword: &'static str, #[case] expected: usize) {
		let ends_in_word = keyword.bytes().last().is_some_and(is_word_byte);
		let substring = SubstringKeyword { finder: Finder::new(keyword), index: 0, ends_in_word };
		assert_eq!(substring.count(content.as_bytes(), usize::MAX), expected);
	}

	#[rstest]
	#[case::hash_comment("# comment", "#", true)]
	#[case::indented("\t  # comment", "#", true)]
	#[case::bare_marker("#", "#", true)]
	#[case::directive("#import <Foundation/Foundation.h>", "#", false)]
	#[case::mid_line("printf(\"%ld\", x);", "%", false)]
	#[case::url("see https://example.com", "//", false)]
	fn test_has_line_comment(#[case] content: &str, #[case] marker: &str, #[case] expected: bool) {
		assert_eq!(has_line_comment(content, marker), expected);
	}

	#[rstest]
	#[case::bare_header("undef.h", "#undef STRINGLIB\n#undef STRINGLIB_LEN\n", "C")]
	#[case::bare_c_file("basic.c", "{\n\t\"empty\",\n\t.insns = { BPF_EXIT_INSN() },\n},\n", "C")]
	fn falls_back_to_default_candidate(#[case] filename: &str, #[case] content: &str, #[case] expected: &str) {
		assert_eq!(detect_language_info(filename, Some(content)).map(|l| l.name), Some(expected));
	}

	#[rstest]
	#[case::longer_extension("layout.blade.php", "Blade")]
	#[case::exact_filename("pom.xml", "Maven")]
	fn most_specific_pattern_wins(#[case] filename: &str, #[case] expected: &str) {
		assert_eq!(detect_language_info(filename, Some("")).map(|l| l.name), Some(expected));
	}

	#[test]
	fn repeated_identifier_does_not_make_c_lpc() {
		// CPython's C API talks about the mapping protocol constantly; that shouldn't make a C file LPC.
		let content = format!(
			"#include \"Python.h\"\n\nstatic PyObject *\nf(PyObject *self)\n{{\n{}    return NULL;\n}}\n",
			"    PyMapping_Check(mapping);\n".repeat(40)
		);
		assert_eq!(detect_language_info("dict.c", Some(&content)).map(|l| l.name), Some("C"));
	}

	#[test]
	fn inc_without_keywords_is_unrecognized() {
		// A kbd keymap: its `#` comments alone shouldn't make it BitBake.
		let content = "# Common keys\nkeycode 1 = Escape\nkeycode 2 = one exclam\n";
		assert!(detect_language_info("qwerty-layout.inc", Some(content)).is_none());
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
	fn word_bits_matches_is_word_byte_for_every_byte() {
		for b in 0..=255u8 {
			for position in 0..8 {
				let x = u64::from(b) << (position * 8);
				assert_eq!(word_bits(x) >> position & 1 == 1, is_word_byte(b), "byte {b:#04x} at {position}");
			}
		}
	}

	/// The block tokenizer must find the same words as a plain byte-by-byte scan, including words that cross 64-byte blocks or end the text.
	#[test]
	fn block_tokenizer_matches_byte_scan() {
		fn reference(content: &str) -> Vec<&[u8]> {
			content.as_bytes().split(|&b| !is_word_byte(b)).filter(|word| !word.is_empty()).collect()
		}
		let mut seed: u64 = 0x1234_5678_9ABC_DEF1;
		let mut next = move |bound: usize| {
			seed ^= seed << 13;
			seed ^= seed >> 7;
			seed ^= seed << 17;
			usize::try_from(seed % bound as u64).unwrap()
		};
		let pieces = ["a", "Z", "_", "9", "word", " ", "\n", "(", "é", "--", "x_1"];
		for _ in 0..3000 {
			let mut content = String::new();
			for _ in 0..next(90) {
				content.push_str(pieces[next(pieces.len())]);
			}
			let tokens = tokenize(&content, &[true; 256], usize::MAX);
			let expected = reference(&content);
			assert_eq!(tokens.words, expected, "{content:?}");
			assert_eq!(tokens.any, !expected.is_empty());
			// And the filters: words starting with `a` or `w`, at most 3 long.
			let mut starts = [false; 256];
			starts[usize::from(b'a')] = true;
			starts[usize::from(b'w')] = true;
			let filtered: Vec<_> =
				expected.iter().copied().filter(|w| w.len() <= 3 && starts[usize::from(w[0])]).collect();
			assert_eq!(tokenize(&content, &starts, 3).words, filtered, "{content:?}");
		}
	}

	#[rstest]
	#[case::splits_on_non_alnum("hello_world foo-bar", vec!["hello_world", "foo", "bar"])]
	#[case::empty("", vec![])]
	#[case::only_delimiters("!@#$%^&*()", vec![])]
	#[case::preserves_underscores("__init__ __main__", vec!["__init__", "__main__"])]
	#[case::non_ascii_delimits("caf\u{e9}_x na\u{ef}ve", vec!["caf", "_x", "na", "ve"])]
	fn test_tokenize(#[case] input: &str, #[case] expected: Vec<&str>) {
		let tokens = tokenize(input, &[true; 256], usize::MAX);
		let expected: Vec<_> = expected.iter().map(|token| token.as_bytes()).collect();
		assert_eq!(tokens.words, expected);
		assert_eq!(tokens.any, !expected.is_empty());
	}

	#[rstest]
	#[case::removes_space("#! /usr/bin/env python", "#!/usr/bin/env python")]
	#[case::leaves_compact_unchanged("#!/bin/bash", "#!/bin/bash")]
	#[case::removes_env_split_flag("#!/usr/bin/env -S python3 -u", "#!/usr/bin/env python3 -u")]
	fn test_normalize_shebang(#[case] input: &str, #[case] expected: &str) {
		assert_eq!(&*normalize_shebang(input), expected);
	}

	#[rstest]
	#[case::python("#!/usr/bin/env python3\nprint('hello')", Some("Python"))]
	#[case::bash("#!/bin/bash\necho hello", Some("Bash"))]
	#[case::env_split("#!/usr/bin/env -S deno run --allow-net\nconsole.log(1)", Some("TypeScript"))]
	#[case::versioned("#!/usr/bin/env python3.12\nprint(1)", Some("Python"))]
	#[case::longest_match("#!/usr/bin/env perl6\nsay 1", Some("Raku"))]
	#[case::interpreter_must_end("#!/usr/bin/env nodemon\nx", None)]
	#[case::no_shebang("print('hello')\n# not a shebang", None)]
	#[case::empty("", None)]
	fn test_detect_from_shebang(#[case] content: &str, #[case] expected_name: Option<&str>) {
		let result = detect_from_shebang(content);
		assert_eq!(result.map(|l| l.name), expected_name);
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
