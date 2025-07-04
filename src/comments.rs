use crate::langs;

/// Represents different types of lines in source code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineType {
	Code,
	Comment,
	Blank,
}

/// Tracks block comment state for a language, including nesting.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CommentState {
	in_block_comment: bool,
	block_comment_depth: usize,
}

impl CommentState {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn enter_block(&mut self, nested: bool) {
		self.in_block_comment = true;
		if nested {
			self.block_comment_depth = 1;
		}
	}

	pub fn exit_block(&mut self, nested: bool) {
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

	pub fn enter_nested_block(&mut self) {
		self.block_comment_depth += 1;
	}

	pub fn is_in_comment(&self) -> bool {
		self.in_block_comment
	}
}

/// Classifies a line of code based on language comment syntax.
pub fn classify_line(line: &str, lang_info: &Option<langs::Language>, comment_state: &mut CommentState) -> LineType {
	let trimmed = line.trim();
	if trimmed.is_empty() {
		return LineType::Blank;
	}
	let Some(lang) = lang_info else {
		return LineType::Code;
	};
	let mut line_remainder = trimmed;
	let mut has_code = false;
	if let Some(ref block_comments) = lang.block_comments {
		let nested = lang.nested_blocks.unwrap_or(false);
		while !line_remainder.is_empty() {
			if !comment_state.is_in_comment() {
				if let Some((pos, start_len)) = find_block_comment_start(line_remainder, block_comments) {
					if pos > 0 && !line_remainder[..pos].trim().is_empty() {
						has_code = true;
					}
					line_remainder = &line_remainder[pos + start_len..];
					comment_state.enter_block(nested);
				} else {
					break;
				}
			} else if let Some((pos, len, found_nested_start)) =
				find_block_comment_end_or_nested_start(line_remainder, block_comments, nested)
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
	if let Some(ref line_comments) = lang.line_comments {
		if let Some(pos) = find_line_comment_start(line_remainder, line_comments) {
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
pub fn find_block_comment_start(line: &str, block_comments: &[Vec<String>]) -> Option<(usize, usize)> {
	block_comments
		.iter()
		.filter_map(|block| block.first().and_then(|start| line.find(start).map(|pos| (pos, start.len()))))
		.min_by_key(|(pos, _)| *pos)
}

/// Finds the earliest end of block comment or nested start.
pub fn find_block_comment_end_or_nested_start(
	line: &str,
	block_comments: &[Vec<String>],
	nested: bool,
) -> Option<(usize, usize, bool)> {
	for block in block_comments {
		if let [start, end] = block.as_slice() {
			let start_pos = if nested { line.find(start) } else { None };
			let end_pos = line.find(end);
			match (start_pos, end_pos) {
				(Some(s), Some(e)) if s < e => return Some((s, start.len(), true)),
				(Some(s), None) => return Some((s, start.len(), true)),
				(_, Some(e)) => return Some((e, end.len(), false)),
				_ => continue,
			}
		}
	}
	None
}

/// Finds the position of the earliest line comment start marker.
pub fn find_line_comment_start(line: &str, line_comments: &[String]) -> Option<usize> {
	line_comments.iter().filter_map(|c| line.find(c)).min()
}
