use std::sync::LazyLock;

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};

use crate::langs::{LANGUAGES, Language};
#[derive(Debug)]
pub struct LanguageMatchers {
	pub(crate) line_comments: Option<AhoCorasick>,
	pub(crate) block_comments: Option<BlockCommentMatchers>,
}

#[derive(Debug, Clone, Copy)]
enum BlockPatternKind {
	Start,
	End,
}

#[derive(Debug)]
pub struct BlockCommentMatchers {
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
				BlockPatternKind::Start => {}
			}
		}
		None
	}
}

static LANGUAGE_MATCHERS: LazyLock<Vec<LanguageMatchers>> =
	LazyLock::new(|| LANGUAGES.iter().map(build_language_matchers).collect());

#[inline]
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
