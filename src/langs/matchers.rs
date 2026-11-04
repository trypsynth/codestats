//! Per-language matchers the line classifier uses to find comment markers, string literals and regexes, built the first time a language is counted.

use std::sync::{LazyLock, OnceLock};

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, Input, MatchKind};
use memchr::{memchr, memchr2, memchr3, memmem::Finder};

use super::data::{Heredoc, LANGUAGES, Language};

#[derive(Debug)]
pub struct LanguageMatchers {
	pub(crate) line_comments: Option<AhoCorasick>,
	pub(crate) block_comments: Option<BlockCommentMatchers>,
	pub(crate) strings: Option<StringMatchers>,
	/// Every byte that can begin a comment marker, string or regex literal: a line with none of them, outside a comment or string, is plain code.
	pub(crate) openers: ByteSet,
}

/// A small set of bytes, searched with memchr when it has three or fewer.
#[derive(Debug)]
pub struct ByteSet {
	bytes: Vec<u8>,
	table: [bool; 256],
}

impl ByteSet {
	fn new(bytes: impl IntoIterator<Item = u8>) -> Self {
		let mut bytes: Vec<u8> = bytes.into_iter().collect();
		bytes.sort_unstable();
		bytes.dedup();
		let mut table = [false; 256];
		for &b in &bytes {
			table[usize::from(b)] = true;
		}
		Self { bytes, table }
	}

	/// The bytes in the set.
	pub(crate) fn bytes(&self) -> &[u8] {
		&self.bytes
	}

	/// Whether `haystack` contains any byte of the set.
	#[inline]
	pub(crate) fn any_in(&self, haystack: &[u8]) -> bool {
		self.find(haystack).is_some()
	}

	/// The position of the first byte of the set in `haystack`.
	#[inline]
	pub(crate) fn find(&self, haystack: &[u8]) -> Option<usize> {
		match *self.bytes.as_slice() {
			[] => None,
			[a] => memchr(a, haystack),
			[a, b] => memchr2(a, b, haystack),
			[a, b, c] => memchr3(a, b, c, haystack),
			[a, b, c, ref rest @ ..] if rest.len() <= 6 => find_in_groups([a, b, c], rest, haystack),
			_ => haystack.iter().position(|&b| self.table[usize::from(b)]),
		}
	}

	/// A set with a byte of each of `markers`, so a text with none of the set's bytes contains none of the markers. A marker's punctuation is chosen over its letters and digits, which turn up on nearly every line, and bytes already chosen are reused: Rust's raw strings `r"` and `r#"` are covered by the `"` of its plain strings rather than by `r`.
	fn covering<'a>(markers: impl IntoIterator<Item = &'a str>) -> Self {
		let mut chosen = Vec::new();
		let mut candidates: Vec<Vec<u8>> = markers
			.into_iter()
			.filter(|marker| !marker.is_empty())
			.map(|marker| {
				let bytes = marker.as_bytes();
				let punctuation: Vec<u8> =
					bytes.iter().copied().filter(|b| !b.is_ascii_alphanumeric() && *b != b'_').collect();
				if punctuation.is_empty() { bytes.to_vec() } else { punctuation }
			})
			.collect();
		// Markers with the fewest choices pick first, so the others can reuse what they picked.
		candidates.sort_by_key(Vec::len);
		for bytes in candidates {
			if !bytes.iter().any(|b| chosen.contains(b)) {
				chosen.push(bytes[0]);
			}
		}
		Self::new(chosen)
	}
}

/// The size of the first window [`find_in_groups`] looks through, longer than nearly every line.
const FIRST_WINDOW: usize = 4096;

/// The first of `first` or `rest` in `haystack`. Kept out of line so the one- to three-byte searches inline.
///
/// It looks through windows that grow, since a byte the rest of the file doesn't have would otherwise be searched for up to its end each time, making a file of the other bytes take quadratic time.
#[inline(never)]
fn find_in_groups(first: [u8; 3], rest: &[u8], haystack: &[u8]) -> Option<usize> {
	// Most searches are of a line.
	if haystack.len() <= FIRST_WINDOW {
		return find_in_window(first, rest, haystack);
	}
	let mut start = 0;
	let mut window = FIRST_WINDOW;
	while start < haystack.len() {
		let end = haystack.len().min(start.saturating_add(window));
		if let Some(found) = find_in_window(first, rest, &haystack[start..end]) {
			return Some(start + found);
		}
		start = end;
		window = window.saturating_mul(2);
	}
	None
}

/// The first of `first` or `rest` in `haystack`, searching for three bytes at a time, each search stopping at the earliest match so far: two or three vectorized passes beat looking at every byte.
#[inline]
fn find_in_window([a, b, c]: [u8; 3], rest: &[u8], haystack: &[u8]) -> Option<usize> {
	let mut found = memchr3(a, b, c, haystack);
	for group in rest.chunks(3) {
		let end = found.unwrap_or(haystack.len());
		let earlier = match *group {
			[a] => memchr(a, &haystack[..end]),
			[a, b] => memchr2(a, b, &haystack[..end]),
			[a, b, c] => memchr3(a, b, c, &haystack[..end]),
			_ => unreachable!("chunks of three are never empty or longer"),
		};
		found = earlier.or(found);
	}
	found
}

/// The language's one-byte strings that close with the byte they open with (`"`, `'`), and the tokens that aren't such strings. A line holding none of those other tokens, whose strings of this kind all close on it, is plain code, and leaves nothing open.
#[derive(Debug)]
pub struct QuickLines {
	others: TokenStarts,
	/// The openers of interpolation holes (`${`), searched for on their own, as a byte like `$` is common in code.
	holes: Vec<&'static [u8]>,
	quotes: ByteSet,
	/// Each quote byte's string, as an index into the language's strings, and its escape.
	strings: Vec<Option<(usize, Option<u8>)>>,
}

impl QuickLines {
	fn new(lang: &Language, tokens: &[(&'static str, Token)]) -> Option<Self> {
		if !lang.line_block_comments.is_empty() || !lang.column_one_comments.is_empty() {
			return None;
		}
		let mut strings = vec![None; 256];
		let mut others = Vec::new();
		for &(token, kind) in tokens {
			match kind {
				Token::String(index) => {
					let delimiter = &lang.strings[index];
					let simple = token.len() == 1
						&& delimiter.close == delimiter.open
						&& !delimiter.doc
						&& !delimiter.char_literal
						&& !delimiter.attribute;
					if simple && strings[usize::from(token.as_bytes()[0])].is_none() {
						strings[usize::from(token.as_bytes()[0])] = Some((index, delimiter.escape));
						continue;
					}
					others.push((token, kind));
				}
				_ => others.push((token, kind)),
			}
		}
		// A line holding a hole opener needs the classifier, which follows the hole.
		let mut holes: Vec<&'static [u8]> = Vec::new();
		for hole in lang.strings.iter().filter_map(|delimiter| delimiter.interpolation) {
			if !holes.contains(&hole.as_bytes()) {
				holes.push(hole.as_bytes());
			}
		}
		let quotes = ByteSet::new((0..=255).filter(|&b| strings[usize::from(b)].is_some()));
		if quotes.bytes.is_empty() {
			return None;
		}
		Some(Self { others: TokenStarts::new(others)?, holes, quotes, strings })
	}

	/// Whether `line`, read from a clear state, is plain code that leaves nothing open: its only tokens are strings of the simple kind, each closing on the line. `false` means the line needs the classifier, not that it isn't code.
	pub(crate) fn is_plain_code(&self, line: &[u8], matchers: &StringMatchers) -> bool {
		let has_hole = |hole: &&[u8]| memchr::memchr_iter(hole[0], line).any(|at| starts_with(&line[at..], hole));
		if self.others.find(line, 0).is_some() || self.holes.iter().any(has_hole) {
			return false;
		}
		let mut pos = 0;
		while let Some(found) = self.quotes.find(&line[pos..]) {
			let at = pos + found;
			let Some((index, escape)) = self.strings[usize::from(line[at])] else { return false };
			// As in the classifier, an escaped quote outside a string opens nothing.
			if at > 0 && (escape == Some(line[at - 1]) || matchers.escapes.contains(&line[at - 1])) {
				pos = at + 1;
				continue;
			}
			let Some(end) = matchers.find_close(line, at + 1, index, escape) else { return false };
			pos = end;
		}
		true
	}
}

/// `haystack.starts_with(needle)` for a needle of a few bytes, compared inline rather than through a call to `memcmp`.
#[inline]
fn starts_with(haystack: &[u8], needle: &[u8]) -> bool {
	haystack.len() >= needle.len() && haystack.iter().zip(needle).all(|(a, b)| a == b)
}

/// A language's tokens by their first byte, each byte's longest first.
#[derive(Debug)]
struct TokenStarts {
	first: ByteSet,
	by_first: Vec<Vec<(&'static [u8], Token)>>,
}

impl TokenStarts {
	/// `None` when a token starts with a letter or digit (Rust's `r"`), whose byte is on nearly every line and would stop the search at every word.
	fn new(tokens: impl IntoIterator<Item = (&'static str, Token)>) -> Option<Self> {
		let mut by_first: Vec<Vec<(&'static [u8], Token)>> = vec![Vec::new(); 256];
		for (token, kind) in tokens {
			let &first = token.as_bytes().first()?;
			if first.is_ascii_alphanumeric() || first == b'_' {
				return None;
			}
			by_first[usize::from(first)].push((token.as_bytes(), kind));
		}
		for bucket in &mut by_first {
			// Longest first; a stable sort keeps the earlier of equal tokens, as the automaton would.
			bucket.sort_by_key(|(token, _)| std::cmp::Reverse(token.len()));
		}
		let first = ByteSet::new((0..=255).filter(|&b| !by_first[usize::from(b)].is_empty()));
		Some(Self { first, by_first })
	}

	#[inline]
	fn find(&self, line: &[u8], mut from: usize) -> Option<(usize, usize, Token)> {
		while let Some(found) = self.first.find(&line[from..]) {
			let at = from + found;
			let rest = &line[at..];
			for &(token, kind) in &self.by_first[usize::from(line[at])] {
				// Byte by byte, as tokens are a few bytes long, which saves a call to `memcmp`.
				if rest.len() >= token.len() && rest.iter().zip(token).all(|(a, b)| a == b) {
					return Some((at, at + token.len(), kind));
				}
			}
			from = at + 1;
		}
		None
	}
}

/// What a token found by [`StringMatchers::find_token`] starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Token {
	LineComment(usize),
	BlockComment(usize),
	String(usize),
	/// The opener of a string whose writer picks its closer (C++'s `R"xy(`), as an index into the language's strings.
	DelimitedString(usize),
	/// A `/` that may start a regex literal (or be division).
	Regex,
	/// A heredoc's opener (`<<`), whose body starts on the next line, if it's followed by a terminator.
	Heredoc,
}

/// Matchers for languages with string literals: one automaton finds whichever comment marker or string opener comes first, so markers inside strings are skipped.
#[derive(Debug)]
pub struct StringMatchers {
	tokens: AhoCorasick,
	/// For each string that spans lines, a byte of what would end it or open a hole in it.
	multiline_ending: Vec<Option<ByteSet>>,
	/// A cheap test for lines that are plain code although they hold strings, so they needn't go through the classifier.
	pub(crate) quick: Option<QuickLines>,
	/// The same tokens by their first byte, where every token starts with punctuation, so the next one is found by a vectorized search for those bytes and a look at the few tokens starting with the one found. On short lines that's much cheaper than the automaton's setup.
	starts: Option<TokenStarts>,
	kinds: Vec<Token>,
	closes: Vec<Finder<'static>>,
	/// First bytes of string openers, for a fast check that a line can't contain a string.
	open_bytes: ByteSet,
	/// The characters that escape inside any of the language's strings (a backslash in a shell, a backtick in `PowerShell`), leaving out quotes that escape by doubling. Outside a string they keep a quote after them from opening one, as `\\'` writes an apostrophe in a shell.
	pub(crate) escapes: Vec<u8>,
}

impl StringMatchers {
	/// The bytes a line would need to end the multi-line string `index`, or open a hole in it; `None` for a string that ends with its line.
	pub(crate) fn multiline_ending_bytes(&self, index: usize) -> Option<&ByteSet> {
		self.multiline_ending[index].as_ref()
	}

	/// Whether `line` contains any byte that could open a string.
	#[inline]
	pub(crate) fn may_contain_string(&self, line: &[u8]) -> bool {
		self.open_bytes.any_in(line)
	}

	/// Find the first comment marker or string opener at or after `from`, preferring the longest at a position (`"""` over `"`).
	#[inline]
	pub(crate) fn find_token(&self, line: &str, from: usize) -> Option<(usize, usize, Token)> {
		if let Some(starts) = &self.starts {
			return starts.find(line.as_bytes(), from);
		}
		self.tokens
			.find(Input::new(line).span(from..line.len()))
			.map(|m| (m.start(), m.end(), self.kinds[m.pattern().as_usize()]))
	}

	/// Find where the string `index` closes, starting at `from` and honoring its escape character. Returns the position just past the closing delimiter.
	#[inline]
	pub(crate) fn find_close(&self, line: &[u8], from: usize, index: usize, escape: Option<u8>) -> Option<usize> {
		let close = self.closes[index].needle();
		self.find_close_start(line, from, index, escape).map(|end| {
			// A tripled quote closes at the end of a longer run of quotes, the ones before it being the string's last characters (`""""` ends a string with a quote).
			let run = close.len() >= 3 && close.iter().all(|&b| b == close[0]);
			if run { end + line[end..].iter().take_while(|&&b| b == close[0]).count() } else { end }
		})
	}

	/// In an interpolated string, where the string `index` closes or a hole (`${`) opens, whichever comes first, starting at `from`: the position just past it, and whether it's a hole. A doubled one-character opener (C#'s `{{`) is a literal brace.
	pub(crate) fn find_close_or_hole(
		&self,
		line: &[u8],
		from: usize,
		index: usize,
		escape: Option<u8>,
		hole: &[u8],
	) -> Option<(usize, bool)> {
		let close = self.closes[index].needle();
		let mut pos = from;
		loop {
			// An escape at the end of the line steps past it.
			let rest = line.get(pos..)?;
			let next = pos
				+ match escape {
					Some(escape) => memchr3(close[0], escape, hole[0], rest)?,
					None => memchr2(close[0], hole[0], rest)?,
				};
			let found = line[next];
			// The hole is checked first, as Swift's `\(` starts with its strings' escape.
			if starts_with(&line[next..], hole) {
				if hole.len() == 1 && line.get(next + 1) == Some(&hole[0]) {
					pos = next + 2;
					continue;
				}
				return Some((next + hole.len(), true));
			}
			if Some(found) == escape {
				// When the escape is the closing quote itself (C#'s `$@"a""b"`), only a doubled quote is an escape.
				let doubled = found == close[0] && line.get(next + 1) == Some(&found);
				if found != close[0] || doubled {
					pos = next + 2;
					continue;
				}
			}
			if starts_with(&line[next..], close) {
				let end = next + close.len();
				let run = close.len() >= 3 && close.iter().all(|&b| b == close[0]);
				return Some((
					if run { end + line[end..].iter().take_while(|&&b| b == close[0]).count() } else { end },
					false,
				));
			}
			pos = next + 1;
		}
	}

	fn find_close_start(&self, line: &[u8], from: usize, index: usize, escape: Option<u8>) -> Option<usize> {
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
	/// For each pair, a byte of what would end or nest it: the last of its end, as the first of `*/` is on every line of many comments.
	ending_bytes: Vec<ByteSet>,
}

/// A single block comment delimiter pair. Once a comment is open, only its own end (and its own start, if it nests) matter.
#[derive(Debug)]
struct BlockCommentPair {
	start: Finder<'static>,
	end: Finder<'static>,
	nests: bool,
}

impl BlockCommentMatchers {
	/// The bytes a line would need to end the comment `pair` opened, or nest another in it.
	pub(crate) fn ending_bytes(&self, pair: usize) -> &ByteSet {
		&self.ending_bytes[pair]
	}

	/// Find the first block comment start, returning its position, length, and pair index.
	#[inline]
	pub(crate) fn find_block_start(&self, line: &str) -> Option<(usize, usize, usize)> {
		self.start_automaton.find(line).map(|m| (m.start(), m.len(), m.pattern().as_usize()))
	}

	/// Find the end of the open comment from `pair`, or a nested start if that pair nests. Returns the position, length, and whether it was a nested start.
	#[inline]
	pub(crate) fn find_block_end_or_nested_start(
		&self,
		line: &str,
		index: usize,
		search: &mut NestedSearch,
	) -> Option<(usize, usize, bool)> {
		let pair = &self.pairs[index];
		let bytes = line.as_bytes();
		if !pair.nests {
			return pair.end.find(bytes).map(|pos| (pos, pair.end.needle().len(), false));
		}
		if search.pair != index {
			*search = NestedSearch { pair: index, ..NestedSearch::default() };
		}
		let end_match = search.end.find(&pair.end, bytes).map(|pos| (pos, pair.end.needle().len(), false));
		let start_match = search.start.find(&pair.start, bytes).map(|pos| (pos, pair.start.needle().len(), true));
		match (start_match, end_match) {
			(Some(s), Some(e)) => Some(if s.0 < e.0 { s } else { e }),
			(s, e) => s.or(e),
		}
	}
}

/// What's known of the next match in the rest of a line, which only ever gets shorter.
#[derive(Clone, Copy, Default)]
enum NextMatch {
	#[default]
	Unknown,
	Absent,
	/// How far the match is from the end of the line.
	BeforeEnd(usize),
}

impl NextMatch {
	#[inline]
	fn find(&mut self, finder: &Finder<'_>, rest: &[u8]) -> Option<usize> {
		match *self {
			Self::Absent => None,
			Self::BeforeEnd(distance) if distance <= rest.len() => Some(rest.len() - distance),
			_ => {
				let found = finder.find(rest);
				*self = found.map_or(Self::Absent, |pos| Self::BeforeEnd(rest.len() - pos));
				found
			}
		}
	}
}

/// Remembers matches between searches of one line, so that a line of many nested openers isn't searched to its end at each one.
#[derive(Clone, Copy, Default)]
pub struct NestedSearch {
	pair: usize,
	end: NextMatch,
	start: NextMatch,
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
		let ending_bytes = lang
			.block_comments
			.iter()
			.map(|&(start, end, nests)| {
				let last = |marker: &str| marker.as_bytes().last().copied();
				ByteSet::new(last(end).into_iter().chain(last(start).filter(|_| nests)))
			})
			.collect();
		Some(BlockCommentMatchers { start_automaton, pairs, ending_bytes })
	};
	let strings = (!lang.strings.is_empty() || lang.heredoc.is_some()).then(|| build_string_matchers(lang));
	let openers = ByteSet::covering(
		lang.line_comments
			.iter()
			.copied()
			.chain(lang.block_comments.iter().map(|&(start, _, _)| start))
			.chain(lang.line_block_comments.iter().map(|&(start, _)| start))
			.chain(lang.strings.iter().map(|delimiter| delimiter.open))
			.chain(lang.heredoc.map(Heredoc::opener))
			.chain(lang.regex_literals.then_some("/")),
	);
	LanguageMatchers { line_comments, block_comments, strings, openers }
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
		kinds.push(if delimiter.delimited { Token::DelimitedString(i) } else { Token::String(i) });
	}
	if lang.regex_literals {
		// Leftmost-longest matching lets `//` and `/*` win over a lone `/`.
		patterns.push("/");
		kinds.push(Token::Regex);
	}
	if let Some(heredoc) = lang.heredoc {
		patterns.push(heredoc.opener());
		kinds.push(Token::Heredoc);
	}
	let starts = TokenStarts::new(patterns.iter().copied().zip(kinds.iter().copied()));
	let all: Vec<(&'static str, Token)> = patterns.iter().copied().zip(kinds.iter().copied()).collect();
	let quick = QuickLines::new(lang, &all);
	let tokens = AhoCorasickBuilder::new()
		.match_kind(MatchKind::LeftmostLongest)
		.build(&patterns)
		.expect("AhoCorasick should never fail to build with valid comment and string patterns");
	let closes = lang.strings.iter().map(|delimiter| Finder::new(delimiter.close)).collect();
	let open_bytes = ByteSet::covering(
		lang.strings
			.iter()
			.map(|delimiter| delimiter.open)
			.chain(lang.heredoc.map(Heredoc::opener))
			.chain(lang.regex_literals.then_some("/")),
	);
	let mut escapes: Vec<u8> = lang
		.strings
		.iter()
		.filter_map(|delimiter| delimiter.escape)
		.filter(|escape| !lang.strings.iter().any(|delimiter| delimiter.open.as_bytes().contains(escape)))
		.collect();
	escapes.sort_unstable();
	escapes.dedup();
	let multiline_ending = lang
		.strings
		.iter()
		.map(|delimiter| {
			delimiter.multiline.then(|| {
				let last_of_close = delimiter.close.as_bytes().last().copied();
				ByteSet::new(
					last_of_close
						.into_iter()
						.chain(delimiter.interpolation.and_then(|hole| hole.as_bytes().first().copied())),
				)
			})
		})
		.collect();
	StringMatchers { tokens, multiline_ending, quick, starts, kinds, closes, open_bytes, escapes }
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::langs::LANGUAGES;

	#[test]
	fn byte_set_finds_bytes_in_any_window() {
		let set = ByteSet::new(*b"abcde");
		for at in [0, 1, FIRST_WINDOW - 1, FIRST_WINDOW, FIRST_WINDOW + 1, 3 * FIRST_WINDOW, 3 * FIRST_WINDOW + 7] {
			for byte in *b"ae" {
				let mut text = vec![b'x'; 4 * FIRST_WINDOW];
				text[at] = byte;
				text[at + 100..].fill(b'c');
				assert_eq!(set.find(&text), Some(at));
			}
		}
		assert_eq!(set.find(&[b'x'; 4 * FIRST_WINDOW]), None);
		assert_eq!(set.find(&[]), None);
	}

	/// The search by first byte must find what the automaton finds, for every language that uses it.
	#[test]
	fn token_starts_find_what_the_automaton_finds() {
		let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
		let mut next = |bound: usize| {
			seed ^= seed << 13;
			seed ^= seed >> 7;
			seed ^= seed << 17;
			usize::try_from(seed % bound as u64).unwrap()
		};
		let mut checked = 0;
		for language in LANGUAGES {
			let Some(strings) = &language_matchers(language).strings else { continue };
			let Some(starts) = &strings.starts else { continue };
			let mut pieces: Vec<&str> =
				vec!["x", " ", "\\", "é", "=", "<", "/", "*", "#", "-", "'", "\"", "`", "(", ")"];
			pieces.extend(language.line_comments);
			pieces.extend(language.block_comments.iter().flat_map(|(start, end, _)| [*start, *end]));
			pieces.extend(language.strings.iter().flat_map(|delimiter| [delimiter.open, delimiter.close]));
			for _ in 0..200 {
				let line: String = (0..next(12)).map(|_| pieces[next(pieces.len())]).collect();
				let mut from = 0;
				while from <= line.len() {
					let expected = strings
						.tokens
						.find(Input::new(&line).span(from..line.len()))
						.map(|m| (m.start(), m.end(), strings.kinds[m.pattern().as_usize()]));
					assert_eq!(starts.find(line.as_bytes(), from), expected, "{}: {line:?} from {from}", language.name);
					from = expected.map_or(line.len() + 1, |(start, _, _)| start + 1);
					while !line.is_char_boundary(from.min(line.len())) {
						from += 1;
					}
				}
			}
			checked += 1;
		}
		assert!(checked > 50, "only {checked} languages");
	}
}
