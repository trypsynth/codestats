use memchr::{memchr2, memrchr};

use crate::langs::{Language, language_matchers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineType {
	Code,
	Comment,
	Blank,
	Shebang,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CommentState {
	in_block_comment: bool,
	block_comment_depth: usize,
}

impl CommentState {
	#[must_use]
	pub(crate) fn new() -> Self {
		Self::default()
	}

	#[inline]
	const fn enter_block(&mut self, nested: bool) {
		self.in_block_comment = true;
		if nested {
			self.block_comment_depth = 1;
		}
	}

	#[inline]
	const fn exit_block(&mut self, nested: bool) {
		if nested {
			self.block_comment_depth = self.block_comment_depth.saturating_sub(1);
			if self.block_comment_depth == 0 {
				self.in_block_comment = false;
			}
		} else {
			self.in_block_comment = false;
			self.block_comment_depth = 0;
		}
	}

	#[inline]
	const fn enter_nested_block(&mut self) {
		self.block_comment_depth += 1;
	}

	#[must_use]
	const fn is_in_comment(&self) -> bool {
		self.in_block_comment
	}
}

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
	if is_first_line && trimmed.starts_with("#!") {
		if let Some(lang) = lang_info {
			if !lang.shebangs.is_empty() && lang.shebangs.iter().any(|shebang| trimmed.starts_with(shebang)) {
				return LineType::Shebang;
			}
		}
	}
	let Some(lang) = lang_info else {
		return LineType::Code;
	};
	let mut line_remainder: &str = trimmed;
	let matchers = language_matchers(lang);
	let mut has_code = false;
	if let Some(block_comments) = matchers.block_comments.as_ref() {
		let nested = lang.nested_blocks;
		while !line_remainder.is_empty() {
			if !comment_state.is_in_comment() {
				if let Some((pos, start_len)) = block_comments.find_block_start(line_remainder) {
					if pos > 0 && contains_non_whitespace(&line_remainder[..pos]) {
						has_code = true;
					}
					line_remainder = &line_remainder[pos + start_len..];
					comment_state.enter_block(nested);
				} else {
					break;
				}
			} else if let Some((pos, len, found_nested_start)) =
				block_comments.find_block_end_or_nested_start(line_remainder, nested)
			{
				if found_nested_start {
					comment_state.enter_nested_block();
				} else {
					comment_state.exit_block(nested);
				}
				line_remainder = &line_remainder[pos + len..];
			} else {
				break;
			}
		}
	}
	if comment_state.is_in_comment() {
		return if has_code { LineType::Code } else { LineType::Comment };
	}
	if let Some(line_comments) = matchers.line_comments.as_ref() {
		if let Some(matched) = line_comments.find(line_remainder) {
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

#[inline]
fn trim_ascii(line: &str) -> &str {
	let bytes = line.as_bytes();
	let mut start = 0;
	let mut end = bytes.len();
	if let Some(pos) = memrchr(b'\n', &bytes[..end]) {
		if pos + 1 == end {
			end = pos;
			if end > 0 && bytes[end - 1] == b'\r' {
				end -= 1;
			}
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
		if !is_ascii_ws(bytes[idx]) {
			return true;
		}
		// Skip runs of common whitespace quickly.
		if let Some(pos) = memchr2(b' ', b'\t', &bytes[idx..]) {
			idx += pos + 1;
		} else {
			idx = bytes.len();
		}
	}
	false
}

#[inline]
const fn is_ascii_ws(b: u8) -> bool {
	matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0B | 0x0C)
}
