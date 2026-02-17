use std::borrow::Cow;

use memchr::{memchr2, memrchr};

use crate::langs::{
	Language,
	scoring::{BlockCommentMatchers, language_matchers},
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

/// Tracks nested block comment state across lines.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CommentState {
	block_comment_depth: usize,
}

impl CommentState {
	#[must_use]
	#[inline]
	pub(crate) fn new() -> Self {
		Self::default()
	}

	#[inline]
	const fn enter_first_block(&mut self) {
		self.block_comment_depth = 1;
	}

	#[inline]
	const fn exit_block(&mut self, nested: bool) {
		if nested {
			self.block_comment_depth = self.block_comment_depth.saturating_sub(1);
		} else {
			self.block_comment_depth = 0;
		}
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
}

/// Process block comments on a line, updating state and detecting code.
/// Returns: (`remaining_line_portion`, `has_code_outside_comments`)
#[inline]
fn handle_block_comments<'a>(
	line: &'a str,
	matchers: &BlockCommentMatchers,
	comment_state: &mut CommentState,
	nested: bool,
) -> (&'a str, bool) {
	let mut line_remainder = line;
	let mut has_code = false;
	while !line_remainder.is_empty() {
		if !comment_state.is_in_comment() {
			if let Some((pos, start_len)) = matchers.find_block_start(line_remainder) {
				if pos > 0 && contains_non_whitespace(&line_remainder[..pos]) {
					has_code = true;
				}
				line_remainder = &line_remainder[pos + start_len..];
				comment_state.enter_first_block();
			} else {
				break;
			}
		} else if let Some((pos, len, found_nested_start)) =
			matchers.find_block_end_or_nested_start(line_remainder, nested)
		{
			if nested && found_nested_start {
				comment_state.enter_nested_block();
			} else {
				comment_state.exit_block(nested);
			}
			line_remainder = &line_remainder[pos + len..];
		} else {
			break;
		}
	}
	(line_remainder, has_code)
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
	let mut line_remainder: &str = trimmed;
	let matchers = language_matchers(lang);
	#[expect(clippy::option_if_let_else)]
	let mut has_code = if let Some(block_comments) = matchers.block_comments.as_ref() {
		let (remainder, found_code) = handle_block_comments(trimmed, block_comments, comment_state, lang.nested_blocks);
		line_remainder = remainder;
		found_code
	} else {
		false
	};
	if comment_state.is_in_comment() {
		return if has_code { LineType::Code } else { LineType::Comment };
	}
	if let Some(line_comments) = matchers.line_comments.as_ref() {
		for matched in line_comments.find_iter(line_remainder) {
			let token = lang.line_comments[matched.pattern().as_usize()];
			if !is_valid_line_comment_match(line_remainder, matched.end(), token) {
				continue;
			}
			let pos = matched.start();
			if pos > 0 && contains_non_whitespace(&line_remainder[..pos]) {
				has_code = true;
			}
			return if has_code { LineType::Code } else { LineType::Comment };
		}
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
fn is_valid_line_comment_match(line: &str, end: usize, token: &str) -> bool {
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
		state.enter_first_block();
		assert!(state.is_in_comment());
		state.enter_nested_block();
		assert!(state.is_in_comment());
		state.exit_block(true); // nested exit
		assert!(state.is_in_comment());
		state.exit_block(true);
		assert!(!state.is_in_comment());
	}

	#[test]
	fn test_comment_state_non_nested_exit() {
		let mut state = CommentState::new();
		state.enter_first_block();
		state.enter_nested_block();
		state.exit_block(false); // non-nested clears all
		assert!(!state.is_in_comment());
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
