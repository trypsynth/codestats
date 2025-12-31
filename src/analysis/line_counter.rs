use std::path::Path;

use anyhow::Result;

use super::{
	encoding::{decode_bytes, FileEncoding},
	file_io::LineSource,
	line_classifier::{self, CommentState, LineType},
	stats::{AnalysisResults, FileContribution, FileStats},
};
use crate::langs::Language;

#[derive(Default)]
pub(super) struct LineCounts {
	pub(super) total: u64,
	pub(super) code: u64,
	pub(super) comment: u64,
	pub(super) blank: u64,
	pub(super) shebang: u64,
}

impl LineCounts {
	pub(super) fn classify_and_count(
		&mut self,
		line: &str,
		lang_info: Option<&Language>,
		comment_state: &mut CommentState,
		is_first_line: bool,
	) {
		let line_type = line_classifier::classify_line(line, lang_info, comment_state, is_first_line);
		match line_type {
			LineType::Code => self.code += 1,
			LineType::Comment => self.comment += 1,
			LineType::Blank => self.blank += 1,
			LineType::Shebang => self.shebang += 1,
		}
		self.total += 1;
	}
}

pub(super) fn process_lines<S>(
	file_path: &Path,
	file_size: u64,
	results: &mut AnalysisResults,
	collect_details: bool,
	language: &'static Language,
	encoding: FileEncoding,
	source: &mut S,
) -> Result<()>
where
	S: LineSource,
{
	let mut is_first_line = true;
	let line_counts = count_lines_with(
		|handle| {
			source.for_each_line(&mut |line_bytes| {
				let decoded = decode_bytes(line_bytes, encoding, is_first_line);
				handle(decoded.as_ref(), is_first_line);
				is_first_line = false;
			})
		},
		language,
	)?;
	finish_file_stats(file_path, file_size, results, collect_details, language, &line_counts);
	Ok(())
}

pub(super) fn finish_file_stats(
	file_path: &Path,
	file_size: u64,
	results: &mut AnalysisResults,
	collect_details: bool,
	language: &'static Language,
	line_counts: &LineCounts,
) {
	let total = line_counts.total;
	let code = line_counts.code;
	let comment = line_counts.comment;
	let blank = line_counts.blank;
	let shebang = line_counts.shebang;
	let contribution = FileContribution::new(total, code, comment, blank, shebang, file_size);
	let file_stats = collect_details
		.then(|| FileStats::new(file_path.display().to_string(), total, code, comment, blank, shebang, file_size));
	results.add_file_stats(language, contribution, file_stats);
}

fn count_lines_with(
	mut for_each: impl FnMut(&mut dyn FnMut(&str, bool)) -> Result<()>,
	language: &'static Language,
) -> Result<LineCounts> {
	let mut line_counts = LineCounts::default();
	let mut comment_state = CommentState::new();
	for_each(&mut |line, is_first_line| {
		line_counts.classify_and_count(line, Some(language), &mut comment_state, is_first_line);
	})?;
	Ok(line_counts)
}
