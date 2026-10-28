use std::sync::{LazyLock, OnceLock};

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, Input, MatchKind};
use memchr::{memchr, memchr2, memchr3, memmem::Finder};

use super::super::data::{LANGUAGES, Language};

#[derive(Debug)]
pub struct LanguageMatchers {
	pub(crate) line_comments: Option<AhoCorasick>,
	pub(crate) block_comments: Option<BlockCommentMatchers>,
	pub(crate) strings: Option<StringMatchers>,
}

/// What a token found by [`StringMatchers::find_token`] starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
	LineComment(usize),
	BlockComment(usize),
	String(usize),
}

/// Matchers for languages with string literals: one automaton finds whichever comment marker or string opener comes first, so markers inside strings are skipped.
#[derive(Debug)]
pub struct StringMatchers {
	tokens: AhoCorasick,
	kinds: Vec<Token>,
	closes: Vec<Finder<'static>>,
	/// Distinct first bytes of string openers, for a fast check that a line can't contain a string.
	open_bytes: Vec<u8>,
	/// Lookup table over `open_bytes`, used when there are more than memchr3 can search for.
	open_table: [bool; 256],
}

impl StringMatchers {
	/// Whether `line` contains any byte that could open a string.
	#[inline]
	pub(crate) fn may_contain_string(&self, line: &[u8]) -> bool {
		match *self.open_bytes.as_slice() {
			[a] => memchr(a, line).is_some(),
			[a, b] => memchr2(a, b, line).is_some(),
			[a, b, c] => memchr3(a, b, c, line).is_some(),
			_ => line.iter().any(|&b| self.open_table[usize::from(b)]),
		}
	}

	/// Find the first comment marker or string opener at or after `from`, preferring the longest at a position (`"""` over `"`).
	#[inline]
	pub(crate) fn find_token(&self, line: &str, from: usize) -> Option<(usize, usize, Token)> {
		self.tokens
			.find(Input::new(line).span(from..line.len()))
			.map(|m| (m.start(), m.end(), self.kinds[m.pattern().as_usize()]))
	}

	/// Find where the string `index` closes, starting at `from` and honoring its escape character. Returns the position just past the closing delimiter.
	#[inline]
	pub(crate) fn find_close(&self, line: &[u8], from: usize, index: usize, escape: Option<u8>) -> Option<usize> {
		let close = &self.closes[index];
		let Some(escape) = escape else {
			return close.find(&line[from..]).map(|pos| from + pos + close.needle().len());
		};
		let first = close.needle()[0];
		let mut pos = from;
		while pos < line.len() {
			let next = pos + memchr2(first, escape, &line[pos..])?;
			// When the escape is the closing quote itself (C#'s `@"a""b"`), only a doubled quote is an escape.
			let doubled = escape == first && line.get(next + 1) == Some(&escape);
			if line[next] == escape && (escape != first || doubled) {
				// Skip the escaped character.
				pos = next + 2;
			} else if line[next..].starts_with(close.needle()) {
				return Some(next + close.needle().len());
			} else {
				pos = next + 1;
			}
		}
		None
	}
}

#[derive(Debug)]
pub struct BlockCommentMatchers {
	start_automaton: AhoCorasick,
	pairs: Vec<BlockCommentPair>,
}

/// A single block comment delimiter pair. Once a comment is open, only its own end (and its own start, if it nests) matter.
#[derive(Debug)]
struct BlockCommentPair {
	start: Finder<'static>,
	end: Finder<'static>,
	nests: bool,
}

impl BlockCommentMatchers {
	/// Find the first block comment start, returning its position, length, and pair index.
	#[inline]
	pub(crate) fn find_block_start(&self, line: &str) -> Option<(usize, usize, usize)> {
		self.start_automaton.find(line).map(|m| (m.start(), m.len(), m.pattern().as_usize()))
	}

	/// Find the end of the open comment from `pair`, or a nested start if that pair nests. Returns the position, length, and whether it was a nested start.
	#[inline]
	pub(crate) fn find_block_end_or_nested_start(&self, line: &str, pair: usize) -> Option<(usize, usize, bool)> {
		let pair = &self.pairs[pair];
		let bytes = line.as_bytes();
		let end_match = pair.end.find(bytes).map(|pos| (pos, pair.end.needle().len(), false));
		if !pair.nests {
			return end_match;
		}
		let start_match = pair.start.find(bytes).map(|pos| (pos, pair.start.needle().len(), true));
		match (start_match, end_match) {
			(Some(s), Some(e)) => Some(if s.0 < e.0 { s } else { e }),
			(s, e) => s.or(e),
		}
	}
}

/// Matchers are built the first time a language is seen, so startup doesn't pay for all ~550 languages.
static LANGUAGE_MATCHERS: LazyLock<Vec<OnceLock<LanguageMatchers>>> =
	LazyLock::new(|| LANGUAGES.iter().map(|_| OnceLock::new()).collect());

#[inline]
#[must_use]
pub fn language_matchers(lang: &Language) -> &'static LanguageMatchers {
	LANGUAGE_MATCHERS[lang.index].get_or_init(|| build_language_matchers(lang))
}

fn build_language_matchers(lang: &Language) -> LanguageMatchers {
	let line_comments = if lang.line_comments.is_empty() {
		None
	} else {
		Some(
			AhoCorasickBuilder::new()
				.match_kind(MatchKind::LeftmostFirst)
				.build(lang.line_comments)
				.expect("AhoCorasick should never fail to build with valid line comment patterns"),
		)
	};
	let block_comments = if lang.block_comments.is_empty() {
		None
	} else {
		let start_automaton = AhoCorasickBuilder::new()
			.match_kind(MatchKind::LeftmostFirst)
			.build(lang.block_comments.iter().map(|(start, _, _)| start))
			.expect("AhoCorasick should never fail to build with valid block comment start patterns");
		let pairs = lang
			.block_comments
			.iter()
			.map(|&(start, end, nests)| BlockCommentPair { start: Finder::new(start), end: Finder::new(end), nests })
			.collect();
		Some(BlockCommentMatchers { start_automaton, pairs })
	};
	let strings = (!lang.strings.is_empty()).then(|| build_string_matchers(lang));
	LanguageMatchers { line_comments, block_comments, strings }
}

fn build_string_matchers(lang: &Language) -> StringMatchers {
	let mut patterns = Vec::new();
	let mut kinds = Vec::new();
	for (i, marker) in lang.line_comments.iter().enumerate() {
		patterns.push(*marker);
		kinds.push(Token::LineComment(i));
	}
	for (i, (start, _, _)) in lang.block_comments.iter().enumerate() {
		patterns.push(*start);
		kinds.push(Token::BlockComment(i));
	}
	for (i, delimiter) in lang.strings.iter().enumerate() {
		patterns.push(delimiter.open);
		kinds.push(Token::String(i));
	}
	let tokens = AhoCorasickBuilder::new()
		.match_kind(MatchKind::LeftmostLongest)
		.build(&patterns)
		.expect("AhoCorasick should never fail to build with valid comment and string patterns");
	let closes = lang.strings.iter().map(|delimiter| Finder::new(delimiter.close)).collect();
	let mut open_bytes: Vec<u8> = lang.strings.iter().map(|delimiter| delimiter.open.as_bytes()[0]).collect();
	open_bytes.sort_unstable();
	open_bytes.dedup();
	let mut open_table = [false; 256];
	for &b in &open_bytes {
		open_table[usize::from(b)] = true;
	}
	StringMatchers { tokens, kinds, closes, open_bytes, open_table }
}
