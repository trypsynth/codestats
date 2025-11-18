use crate::langs::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineType {
	Code,
	Comment,
	Blank,
	Shebang,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct CommentState {
	in_block_comment: bool,
	block_comment_depth: usize,
}

impl CommentState {
	#[must_use]
	pub(crate) fn new() -> Self {
		Self::default()
	}

	#[inline]
	fn enter_block(&mut self, nested: bool) {
		self.in_block_comment = true;
		if nested {
			self.block_comment_depth = 1;
		}
	}

	#[inline]
	fn exit_block(&mut self, nested: bool) {
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
	fn enter_nested_block(&mut self) {
		self.block_comment_depth += 1;
	}

	#[must_use]
	const fn is_in_comment(&self) -> bool {
		self.in_block_comment
	}
}

#[inline]
pub(crate) fn classify_line(
	line: &str,
	lang_info: Option<&Language>,
	comment_state: &mut CommentState,
	is_first_line: bool,
) -> LineType {
	let trimmed = line.trim();
	if trimmed.is_empty() {
		return LineType::Blank;
	}
	if is_first_line && trimmed.starts_with("#!") {
		if let Some(lang) = lang_info {
			if lang.shebangs.is_empty() || lang.shebangs.iter().any(|shebang| trimmed.starts_with(shebang)) {
				return LineType::Shebang;
			}
		} else {
			// For unknown languages, treat #! on the first line as as shebang.
			return LineType::Shebang;
		}
	}
	let Some(lang) = lang_info else {
		return LineType::Code;
	};
	let mut line_remainder = trimmed;
	let mut has_code = false;
	if !lang.block_comments.is_empty() {
		let nested = lang.nested_blocks;
		while !line_remainder.is_empty() {
			if !comment_state.is_in_comment() {
				if let Some((pos, start_len)) = find_block_comment_start(line_remainder, lang.block_comments) {
					if pos > 0 && !line_remainder[..pos].trim().is_empty() {
						has_code = true;
					}
					line_remainder = &line_remainder[pos + start_len..];
					comment_state.enter_block(nested);
				} else {
					break;
				}
			} else if let Some((pos, len, found_nested_start)) =
				find_block_comment_end_or_nested_start(line_remainder, lang.block_comments, nested)
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
	if !lang.line_comments.is_empty() {
		if let Some(pos) = find_line_comment_start(line_remainder, lang.line_comments) {
			if pos > 0 && !line_remainder[..pos].trim().is_empty() {
				has_code = true;
			}
			return if has_code { LineType::Code } else { LineType::Comment };
		}
	}
	if !line_remainder.trim().is_empty() {
		has_code = true;
	}
	if has_code { LineType::Code } else { LineType::Comment }
}

/// Finds the earliest block comment start marker in the line.
#[inline]
fn find_block_comment_start(line: &str, block_comments: &[(&str, &str)]) -> Option<(usize, usize)> {
	block_comments
		.iter()
		.filter_map(|(start, _)| line.find(start).map(|pos| (pos, start.len())))
		.min_by_key(|(pos, _)| *pos)
}

/// Finds the earliest end of block comment or nested start.
#[inline]
fn find_block_comment_end_or_nested_start(
	line: &str,
	block_comments: &[(&str, &str)],
	nested: bool,
) -> Option<(usize, usize, bool)> {
	let mut best: Option<(usize, usize, bool)> = None;
	for (start, end) in block_comments {
		if nested {
			if let Some(pos) = line.find(start) {
				if best.is_none_or(|(best_pos, _, is_nested)| pos < best_pos || (pos == best_pos && !is_nested)) {
					best = Some((pos, start.len(), true));
				}
			}
		}
		if let Some(pos) = line.find(end) {
			if best.is_none_or(|(best_pos, _, is_nested)| pos < best_pos || (pos == best_pos && is_nested)) {
				best = Some((pos, end.len(), false));
			}
		}
	}
	best
}

/// Finds the position of the earliest line comment start marker.
#[inline]
fn find_line_comment_start(line: &str, line_comments: &[&str]) -> Option<usize> {
	line_comments.iter().filter_map(|comment| line.find(comment)).min()
}
