use anyhow::Result;

use super::{
	encoding::{FileEncoding, decode_bytes},
	file_io::LineSource,
	line_classifier::{CommentState, classify_line},
	stats::LineCounts,
};
use crate::langs::Language;

pub(super) fn count_lines<S>(language: &Language, encoding: FileEncoding, source: &mut S) -> Result<LineCounts>
where
	S: LineSource,
{
	let mut counts = LineCounts::default();
	let mut comment_state = CommentState::new();
	let mut is_first_line = true;
	source.for_each_line(&mut |line_bytes| {
		let line = decode_bytes(line_bytes, encoding, is_first_line);
		counts.add(classify_line(&line, Some(language), &mut comment_state, is_first_line));
		is_first_line = false;
	})?;
	Ok(counts)
}
