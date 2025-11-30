use std::sync::LazyLock;

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};

use crate::langs::{LANGUAGES, Language};
#[derive(Debug)]
pub struct LanguageMatchers {
	pub(crate) line_comments: Option<AhoCorasick>,
	pub(crate) block_comments: Option<BlockCommentMatchers>,
}

#[derive(Debug)]
pub struct BlockCommentMatchers {
	start_automaton: AhoCorasick,
	end_automaton: AhoCorasick,
}

impl BlockCommentMatchers {
	#[inline]
	pub(crate) fn find_block_start(&self, line: &str) -> Option<(usize, usize)> {
		self.start_automaton.find(line).map(|m| (m.start(), m.len()))
	}

	#[inline]
	pub(crate) fn find_block_end_or_nested_start(&self, line: &str, nested: bool) -> Option<(usize, usize, bool)> {
		if nested {
			let start_match = self.start_automaton.find(line);
			let end_match = self.end_automaton.find(line);
			match (start_match, end_match) {
				(Some(s), Some(e)) if s.start() < e.start() => Some((s.start(), s.len(), true)),
				(Some(s), None) => Some((s.start(), s.len(), true)),
				(_, Some(e)) => Some((e.start(), e.len(), false)),
				(None, None) => None,
			}
		} else {
			self.end_automaton.find(line).map(|m| (m.start(), m.len(), false))
		}
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
			AhoCorasickBuilder::new().match_kind(MatchKind::LeftmostFirst).build(lang.line_comments).unwrap(),
		)
	};
	let block_comments = if lang.block_comments.is_empty() {
		None
	} else {
		let mut start_patterns = Vec::with_capacity(lang.block_comments.len());
		let mut end_patterns = Vec::with_capacity(lang.block_comments.len());
		for (start, end) in lang.block_comments {
			start_patterns.push(*start);
			end_patterns.push(*end);
		}
		let start_automaton =
			AhoCorasickBuilder::new().match_kind(MatchKind::LeftmostFirst).build(start_patterns).unwrap();
		let end_automaton = AhoCorasickBuilder::new().match_kind(MatchKind::LeftmostFirst).build(end_patterns).unwrap();
		Some(BlockCommentMatchers { start_automaton, end_automaton })
	};
	LanguageMatchers { line_comments, block_comments }
}
