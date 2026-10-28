use std::borrow::Cow;

use memchr::{memchr2, memrchr};

use crate::langs::{
	Language,
	scoring::{BlockCommentMatchers, LanguageMatchers, StringMatchers, Token, language_matchers},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineType {
	Code,
	Comment,
	Blank,
	Shebang,
}

impl LineType {
	#[must_use]
	pub const fn singular_label(self) -> &'static str {
		match self {
			Self::Code => "code",
			Self::Comment => "comment",
			Self::Blank => "blank",
			Self::Shebang => "shebang",
		}
	}

	#[must_use]
	pub const fn plural_label(self) -> &'static str {
		match self {
			Self::Code => "code",
			Self::Comment => "comments",
			Self::Blank => "blanks",
			Self::Shebang => "shebangs",
		}
	}

	#[must_use]
	pub const fn title_label(self) -> &'static str {
		match self {
			Self::Code => "Code",
			Self::Comment => "Comments",
			Self::Blank => "Blanks",
			Self::Shebang => "Shebangs",
		}
	}
}

/// Tracks block comment state across lines: how deeply nested we are, and which delimiter pair opened the comment.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CommentState {
	block_comment_depth: usize,
	pair: usize,
	/// The string literal left open by the previous line, as an index into the language's `strings`.
	string: Option<usize>,
}

impl CommentState {
	#[must_use]
	#[inline]
	pub(crate) fn new() -> Self {
		Self::default()
	}

	#[inline]
	const fn enter_first_block(&mut self, pair: usize) {
		self.block_comment_depth = 1;
		self.pair = pair;
	}

	/// Close one level of comment. Non-nesting pairs never go deeper than one level, so this fully exits them.
	#[inline]
	const fn exit_block(&mut self) {
		self.block_comment_depth = self.block_comment_depth.saturating_sub(1);
	}

	#[inline]
	const fn enter_nested_block(&mut self) {
		self.block_comment_depth = self.block_comment_depth.saturating_add(1);
	}

	#[must_use]
	#[inline]
	const fn is_in_comment(&self) -> bool {
		self.block_comment_depth > 0
	}

	#[must_use]
	#[inline]
	const fn is_in_string(&self) -> bool {
		self.string.is_some()
	}
}

/// Process block comments on a line, updating state and detecting code.
/// Returns: (`remaining_line_portion`, `has_code_outside_comments`)
#[inline]
fn handle_block_comments<'a>(
	line: &'a str,
	lang: &Language,
	matchers: &LanguageMatchers,
	block_matchers: &BlockCommentMatchers,
	comment_state: &mut CommentState,
) -> (&'a str, bool) {
	let mut line_remainder = line;
	let mut has_code = false;
	while !line_remainder.is_empty() {
		if !comment_state.is_in_comment() {
			if let Some((pos, start_len, pair)) = block_matchers.find_block_start(line_remainder) {
				// A block start after a line comment marker is part of that comment (`// see "src/*"`), so leave the line comment for the caller.
				if find_line_comment(line_remainder, lang, matchers).is_some_and(|line_pos| line_pos < pos) {
					break;
				}
				if pos > 0 && contains_non_whitespace(&line_remainder[..pos]) {
					has_code = true;
				}
				line_remainder = &line_remainder[pos + start_len..];
				comment_state.enter_first_block(pair);
			} else {
				break;
			}
		} else if let Some((pos, len, found_nested_start)) =
			block_matchers.find_block_end_or_nested_start(line_remainder, comment_state.pair)
		{
			if found_nested_start {
				comment_state.enter_nested_block();
			} else {
				comment_state.exit_block();
			}
			line_remainder = &line_remainder[pos + len..];
		} else {
			break;
		}
	}
	(line_remainder, has_code)
}

/// Classify a line in a language with string literals, scanning left to right so comment markers inside strings are ignored. String contents count as code.
fn classify_with_strings(
	line: &str,
	lang: &Language,
	matchers: &LanguageMatchers,
	strings: &StringMatchers,
	state: &mut CommentState,
) -> LineType {
	let bytes = line.as_bytes();
	let mut pos = 0;
	let mut has_code = false;
	while pos < line.len() {
		if state.is_in_comment() {
			// Strings aren't lexed inside comments, so only the open comment's own end (or nested start) matters.
			let Some((found, len, nested)) = matchers
				.block_comments
				.as_ref()
				.and_then(|block| block.find_block_end_or_nested_start(&line[pos..], state.pair))
			else {
				break;
			};
			if nested {
				state.enter_nested_block();
			} else {
				state.exit_block();
			}
			pos += found + len;
		} else if let Some(index) = state.string {
			has_code = true;
			let delimiter = &lang.strings[index];
			if let Some(end) = strings.find_close(bytes, pos, index, delimiter.escape) {
				state.string = None;
				pos = end;
			} else {
				// An unclosed single-line string ends with the line, unless the line ends in an escape (a continuation).
				let continues = delimiter.escape.is_some_and(|escape| bytes.last() == Some(&escape));
				if !delimiter.multiline && !continues {
					state.string = None;
				}
				break;
			}
		} else {
			let Some((start, end, token)) = strings.find_token(line, pos) else {
				has_code |= contains_non_whitespace(&line[pos..]);
				break;
			};
			has_code |= contains_non_whitespace(&line[pos..start]);
			pos = end;
			match token {
				Token::LineComment(i) => {
					if is_valid_line_comment_match(line, end, lang.line_comments[i]) {
						return if has_code { LineType::Code } else { LineType::Comment };
					}
					has_code = true;
				}
				Token::BlockComment(pair) => state.enter_first_block(pair),
				Token::String(i) => {
					has_code = true;
					let delimiter = &lang.strings[i];
					// An escaped quote outside a string (in a regex or sigil like `~r[\"]`) doesn't open one.
					let escaped = start > 0 && delimiter.escape == Some(bytes[start - 1]);
					// A quote that doesn't look like a char literal (a Rust lifetime like `'a`) is just code.
					let char_mismatch = delimiter.char_literal && !looks_like_char(bytes, end);
					if !escaped && !char_mismatch {
						state.string = Some(i);
					}
				}
			}
		}
	}
	if has_code { LineType::Code } else { LineType::Comment }
}

/// Whether the text after an opening `'` looks like a char literal: one character or an escape, then a closing `'`.
fn looks_like_char(line: &[u8], start: usize) -> bool {
	match line.get(start) {
		None | Some(b'\'') => false,
		// Escapes like `'\n'` or `'\u{1F600}'`: the escaped character, then a closing quote within a few bytes.
		Some(b'\\') => line.get(start + 2..).is_some_and(|rest| rest.iter().take(10).any(|&b| b == b'\'')),
		Some(&lead) => {
			let len = match lead {
				0x00..=0x7F => 1,
				0xC0..=0xDF => 2,
				0xE0..=0xEF => 3,
				_ => 4,
			};
			line.get(start + len) == Some(&b'\'')
		}
	}
}

/// Find the position of the first valid line comment marker in `line`.
#[inline]
fn find_line_comment(line: &str, lang: &Language, matchers: &LanguageMatchers) -> Option<usize> {
	let line_comments = matchers.line_comments.as_ref()?;
	line_comments
		.find_iter(line)
		.find(|matched| {
			is_valid_line_comment_match(line, matched.end(), lang.line_comments[matched.pattern().as_usize()])
		})
		.map(|matched| matched.start())
}

/// Classify a line as code, comment, blank, or shebang.
#[inline]
pub fn classify_line(
	line: &str,
	lang_info: Option<&Language>,
	comment_state: &mut CommentState,
	is_first_line: bool,
) -> LineType {
	let trimmed = trim_ascii(line);
	if trimmed.is_empty() {
		return LineType::Blank;
	}
	if is_first_line
		&& trimmed.starts_with("#!")
		&& let Some(lang) = lang_info
		&& !lang.shebangs.is_empty()
	{
		// Normalize shebang by removing optional space after `#!`
		let normalized: Cow<'_, str> =
			trimmed.strip_prefix("#! ").map_or(Cow::Borrowed(trimmed), |rest| Cow::Owned(format!("#!{rest}")));
		if lang.shebangs.iter().any(|shebang| normalized.starts_with(shebang)) {
			return LineType::Shebang;
		}
	}
	let Some(lang) = lang_info else {
		return LineType::Code;
	};
	// Markers like fixed-form Fortran's `C` only count in the first column, so check the untrimmed line.
	if !comment_state.is_in_comment()
		&& !comment_state.is_in_string()
		&& lang.column_one_comments.iter().any(|marker| line.starts_with(marker))
	{
		return LineType::Comment;
	}
	let matchers = language_matchers(lang);
	if let Some(strings) = matchers.strings.as_ref()
		&& (comment_state.is_in_string() || strings.may_contain_string(trimmed.as_bytes()))
	{
		return classify_with_strings(trimmed, lang, matchers, strings, comment_state);
	}
	let mut line_remainder = trimmed;
	#[expect(
		clippy::option_if_let_else,
		reason = "the two branches have different types of side effects (mutating line_remainder) that map_or_else cannot express cleanly"
	)]
	let mut has_code = if let Some(block_comments) = matchers.block_comments.as_ref() {
		let (remainder, found_code) = handle_block_comments(trimmed, lang, matchers, block_comments, comment_state);
		line_remainder = remainder;
		found_code
	} else {
		false
	};
	if comment_state.is_in_comment() {
		return if has_code { LineType::Code } else { LineType::Comment };
	}
	if let Some(pos) = find_line_comment(line_remainder, lang, matchers) {
		if pos > 0 && contains_non_whitespace(&line_remainder[..pos]) {
			has_code = true;
		}
		return if has_code { LineType::Code } else { LineType::Comment };
	}
	if contains_non_whitespace(line_remainder) {
		has_code = true;
	}
	if has_code { LineType::Code } else { LineType::Comment }
}

/// Fast ASCII-only whitespace trimming with newline handling. This is a performance-critical hot path called for every line of code analyzed.
///
/// We use a manual byte-based implementation instead of `str::trim()` because:
/// 1. We need to handle trailing \r\n properly (from both Unix and Windows line endings).
/// 2. Byte operations avoid UTF-8 boundary checks since we only trim ASCII whitespace.
/// 3. This is measurably faster in benchmarks for typical source code.
#[inline]
fn trim_ascii(line: &str) -> &str {
	let bytes = line.as_bytes();
	let mut start = 0;
	let mut end = bytes.len();
	if let Some(pos) = memrchr(b'\n', &bytes[..end])
		&& pos + 1 == end
	{
		end = pos;
		if end > 0 && bytes[end - 1] == b'\r' {
			end -= 1;
		}
	}
	while start < end && is_ascii_ws(bytes[start]) {
		start += 1;
	}
	while end > start && is_ascii_ws(bytes[end - 1]) {
		end -= 1;
	}
	&line[start..end]
}

#[inline]
fn contains_non_whitespace(s: &str) -> bool {
	let bytes = s.as_bytes();
	let mut idx = 0;
	while idx < bytes.len() {
		let byte = bytes[idx];
		if !is_ascii_ws(byte) {
			return true;
		}
		if byte == b' ' || byte == b'\t' {
			// Skip runs of common whitespace quickly.
			if let Some(pos) = memchr2(b' ', b'\t', &bytes[idx..]) {
				idx += pos + 1;
			} else {
				idx = bytes.len();
			}
		} else {
			idx += 1;
		}
	}
	false
}

#[inline]
const fn is_ascii_ws(b: u8) -> bool {
	matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}

#[inline]
const fn is_word_char(b: u8) -> bool {
	b.is_ascii_alphanumeric() || b == b'_'
}

#[inline]
const fn is_valid_line_comment_match(line: &str, end: usize, token: &str) -> bool {
	let Some(&first) = token.as_bytes().first() else {
		return false;
	};
	if is_word_char(first) {
		let bytes = line.as_bytes();
		if end < bytes.len() && is_word_char(bytes[end]) {
			return false;
		}
	}
	true
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;

	#[rstest]
	#[case::spaces("  hello  ", "hello")]
	#[case::tabs("\t\tworld\t\t", "world")]
	#[case::only_spaces("   ", "")]
	#[case::empty("", "")]
	#[case::trailing_lf("hello\n", "hello")]
	#[case::trailing_crlf("hello\r\n", "hello")]
	#[case::spaces_trailing_lf("  hello  \n", "hello")]
	#[case::spaces_trailing_crlf("  hello  \r\n", "hello")]
	fn test_trim_ascii(#[case] input: &str, #[case] expected: &str) {
		assert_eq!(trim_ascii(input), expected);
	}

	#[test]
	fn test_contains_non_whitespace() {
		assert!(contains_non_whitespace("hello"));
		assert!(contains_non_whitespace("  x  "));
		assert!(!contains_non_whitespace(""));
		assert!(!contains_non_whitespace("   "));
		assert!(!contains_non_whitespace("\t\t"));
		assert!(contains_non_whitespace("\u{000B}x"));
	}

	#[rstest]
	#[case::empty("")]
	#[case::spaces("   ")]
	#[case::tabs("\t\t")]
	fn test_classify_blank_lines(#[case] line: &str) {
		let mut state = CommentState::new();
		assert_eq!(classify_line(line, None, &mut state, false), LineType::Blank);
	}

	#[rstest]
	#[case("some code")]
	#[case("  more code  ")]
	fn test_classify_code_without_language(#[case] line: &str) {
		let mut state = CommentState::new();
		assert_eq!(classify_line(line, None, &mut state, false), LineType::Code);
	}

	#[test]
	fn test_comment_state_nesting() {
		let mut state = CommentState::new();
		assert!(!state.is_in_comment());
		state.enter_first_block(0);
		assert!(state.is_in_comment());
		state.enter_nested_block();
		assert!(state.is_in_comment());
		state.exit_block();
		assert!(state.is_in_comment());
		state.exit_block();
		assert!(!state.is_in_comment());
	}

	fn classify_all(lang_name: &str, lines: &[&str]) -> Vec<LineType> {
		let lang = crate::langs::LANGUAGES.iter().find(|l| l.name == lang_name).unwrap();
		let mut state = CommentState::new();
		lines.iter().enumerate().map(|(i, line)| classify_line(line, Some(lang), &mut state, i == 0)).collect()
	}

	#[test]
	fn block_comment_only_closes_with_its_own_end() {
		// `*)` must not close a comment opened with `{`.
		let types = classify_all("Pascal", &["{ start", "still *) inside", "} x := 1;", "y := 2;"]);
		assert_eq!(types, [LineType::Comment, LineType::Comment, LineType::Code, LineType::Code]);
	}

	#[rstest]
	#[case::marker_in_string(&["let s = \"/* no\";", "x();"], &[LineType::Code, LineType::Code])]
	#[case::escaped_quote(&["let s = \"\\\"/*\";", "x();"], &[LineType::Code, LineType::Code])]
	#[case::escaped_escape(&["let s = \"\\\\\"; /* c */"], &[LineType::Code])]
	#[case::lifetime_then_comment(&["fn f<'a>(s: &'a str) -> char { '\"' }", "/* c */"], &[LineType::Code, LineType::Comment])]
	#[case::raw_string(&["let r = r#\"*/ \" /*\"#;", "x();"], &[LineType::Code, LineType::Code])]
	#[case::multiline_string(&["let s = \"a", "/* in string */\";", "/* c */"], &[LineType::Code, LineType::Code, LineType::Comment])]
	#[case::quote_in_comment(&["// don't", "x();"], &[LineType::Comment, LineType::Code])]
	fn rust_strings(#[case] lines: &[&str], #[case] expected: &[LineType]) {
		assert_eq!(classify_all("Rust", lines), expected);
	}

	#[rstest]
	#[case::doubled_quotes(&["var s = @\"value=\"\"\"\" />\";", "// c"], &[LineType::Code, LineType::Comment])]
	#[case::verbatim_backslash(&["var p = @\"C:\\temp\\\";", "// c"], &[LineType::Code, LineType::Comment])]
	fn csharp_verbatim_strings(#[case] lines: &[&str], #[case] expected: &[LineType]) {
		assert_eq!(classify_all("C#", lines), expected);
	}

	#[test]
	fn escaped_quote_outside_string_does_not_open_one() {
		let types = classify_all("Elixir", &["assert_raise E, ~r[key with \\\"=\\\"], fn ->", "# c"]);
		assert_eq!(types, [LineType::Code, LineType::Comment]);
	}

	#[test]
	fn unterminated_single_line_string_ends_with_line() {
		let types = classify_all("C", &["char *s = \"abc;", "/* c */"]);
		assert_eq!(types, [LineType::Code, LineType::Comment]);
	}

	#[rstest]
	#[case::plain(b"a'", 0, true)]
	#[case::escape(b"\\n'", 0, true)]
	#[case::unicode_escape(b"\\u{1F600}'", 0, true)]
	#[case::multibyte("é'".as_bytes(), 0, true)]
	#[case::lifetime(b"a>", 0, false)]
	#[case::empty(b"'", 0, false)]
	fn test_looks_like_char(#[case] line: &[u8], #[case] start: usize, #[case] expected: bool) {
		assert_eq!(looks_like_char(line, start), expected);
	}

	#[test]
	fn block_start_inside_line_comment_is_ignored() {
		let types = classify_all("Go", &["// matches \"/healthz/*\" paths", "x := 1", "y := 2"]);
		assert_eq!(types, [LineType::Comment, LineType::Code, LineType::Code]);
	}

	#[rstest]
	#[case::nesting_pair(&["/+ outer", "/+ inner +/", "still comment", "+/ x = 1;"], &[LineType::Comment, LineType::Comment, LineType::Comment, LineType::Code])]
	#[case::non_nesting_pair(&["/* outer", "/* not nested */", "x = 1;"], &[LineType::Comment, LineType::Comment, LineType::Code])]
	fn d_nests_only_plus_comments(#[case] lines: &[&str], #[case] expected: &[LineType]) {
		assert_eq!(classify_all("D", lines), expected);
	}

	#[test]
	fn test_is_valid_line_comment_match() {
		// Word-char tokens need word boundary after
		assert!(is_valid_line_comment_match("REM hello", 3, "REM"));
		assert!(!is_valid_line_comment_match("REMEMBER", 3, "REM")); // no boundary
		// Symbol tokens don't need boundary
		assert!(is_valid_line_comment_match("// test", 2, "//"));
		assert!(is_valid_line_comment_match("//test", 2, "//"));
	}

	#[rstest]
	#[case(b' ', true)]
	#[case(b'\t', true)]
	#[case(b'\n', true)]
	#[case(b'\r', true)]
	#[case(0x0B, true)]
	#[case(0x0C, true)]
	#[case(b'a', false)]
	#[case(b'0', false)]
	fn test_is_ascii_ws(#[case] byte: u8, #[case] expected: bool) {
		assert_eq!(is_ascii_ws(byte), expected);
	}

	#[rstest]
	#[case(b'a', true)]
	#[case(b'Z', true)]
	#[case(b'5', true)]
	#[case(b'_', true)]
	#[case(b' ', false)]
	#[case(b'/', false)]
	#[case(b'-', false)]
	fn test_is_word_char(#[case] byte: u8, #[case] expected: bool) {
		assert_eq!(is_word_char(byte), expected);
	}

	#[test]
	fn test_line_type_labels() {
		assert_eq!(LineType::Code.singular_label(), "code");
		assert_eq!(LineType::Code.plural_label(), "code");
		assert_eq!(LineType::Comment.plural_label(), "comments");
		assert_eq!(LineType::Blank.title_label(), "Blanks");
		assert_eq!(LineType::Shebang.singular_label(), "shebang");
	}
}
