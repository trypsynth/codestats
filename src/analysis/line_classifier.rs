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
		&& lang.shebangs.iter().any(|shebang| trimmed.starts_with(shebang))
	{
		return LineType::Shebang;
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
	if let Some(line_comments) = matchers.line_comments.as_ref()
		&& let Some(matched) = line_comments.find(line_remainder)
	{
		let pos = matched.start();
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_trim_ascii() {
		assert_eq!(trim_ascii("  hello  "), "hello");
		assert_eq!(trim_ascii("\t\tworld\t\t"), "world");
		assert_eq!(trim_ascii("   "), "");
		assert_eq!(trim_ascii(""), "");
		assert_eq!(trim_ascii("hello\n"), "hello");
		assert_eq!(trim_ascii("hello\r\n"), "hello");
		assert_eq!(trim_ascii("  hello  \n"), "hello");
		assert_eq!(trim_ascii("  hello  \r\n"), "hello");
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

	#[test]
	fn test_classify_blank_lines() {
		let mut state = CommentState::new();
		assert_eq!(classify_line("", None, &mut state, false), LineType::Blank);
		assert_eq!(classify_line("   ", None, &mut state, false), LineType::Blank);
		assert_eq!(classify_line("\t\t", None, &mut state, false), LineType::Blank);
	}

	#[test]
	fn test_classify_code_without_language() {
		let mut state = CommentState::new();
		assert_eq!(classify_line("some code", None, &mut state, false), LineType::Code);
		assert_eq!(classify_line("  more code  ", None, &mut state, false), LineType::Code);
	}
}
