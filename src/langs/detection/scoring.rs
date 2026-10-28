use std::sync::LazyLock;

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use memchr::memmem::Finder;

use super::super::data::{LANGUAGES, Language};

#[derive(Debug)]
pub struct LanguageMatchers {
	pub(crate) line_comments: Option<AhoCorasick>,
	pub(crate) block_comments: Option<BlockCommentMatchers>,
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

static LANGUAGE_MATCHERS: LazyLock<Vec<LanguageMatchers>> =
	LazyLock::new(|| LANGUAGES.iter().map(build_language_matchers).collect());

#[inline]
#[must_use]
pub fn language_matchers(lang: &Language) -> &'static LanguageMatchers {
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
	LanguageMatchers { line_comments, block_comments }
}
