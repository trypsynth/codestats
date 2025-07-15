use std::{cmp::Reverse, collections::HashMap};

use crate::utils;

/// Holds statistics about a programming language's usage throughout a project/folder.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LangStats {
	pub(crate) files: u64,
	pub(crate) lines: u64,
	pub(crate) code_lines: u64,
	pub(crate) comment_lines: u64,
	pub(crate) blank_lines: u64,
	pub(crate) size: u64,
}

impl LangStats {
	pub(crate) fn add_file(&mut self, file_stats: FileStats) {
		self.files += 1;
		self.lines += file_stats.total_lines;
		self.code_lines += file_stats.code_lines;
		self.comment_lines += file_stats.comment_lines;
		self.blank_lines += file_stats.blank_lines;
		self.size += file_stats.size;
	}

	pub(crate) fn code_percentage(&self) -> f64 {
		utils::percentage(self.code_lines, self.lines)
	}

	pub(crate) fn comment_percentage(&self) -> f64 {
		utils::percentage(self.comment_lines, self.lines)
	}

	pub(crate) fn blank_percentage(&self) -> f64 {
		utils::percentage(self.blank_lines, self.lines)
	}
}

/// Statistics for a single file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileStats {
	pub(crate) total_lines: u64,
	pub(crate) code_lines: u64,
	pub(crate) comment_lines: u64,
	pub(crate) blank_lines: u64,
	pub(crate) size: u64,
}

impl FileStats {
	pub(crate) fn new(total_lines: u64, code_lines: u64, comment_lines: u64, blank_lines: u64, size: u64) -> Self {
		Self { total_lines, code_lines, comment_lines, blank_lines, size }
	}
}

/// Thread-safe statistics collector
#[derive(Debug, Default)]
pub struct StatsCollector {
	pub(crate) total_files: u64,
	pub(crate) total_lines: u64,
	pub(crate) total_code_lines: u64,
	pub(crate) total_comment_lines: u64,
	pub(crate) total_blank_lines: u64,
	pub(crate) total_size: u64,
	pub(crate) lang_stats: HashMap<String, LangStats>,
}

impl StatsCollector {
	pub(crate) fn add_file_stats(&mut self, language: String, file_stats: FileStats) {
		self.total_files += 1;
		self.total_lines += file_stats.total_lines;
		self.total_code_lines += file_stats.code_lines;
		self.total_comment_lines += file_stats.comment_lines;
		self.total_blank_lines += file_stats.blank_lines;
		self.total_size += file_stats.size;
		self.lang_stats.entry(language).or_default().add_file(file_stats);
	}

	/// Returns languages sorted by line count (descending)
	pub(crate) fn languages_by_lines(&self) -> Vec<(&String, &LangStats)> {
		let mut stats_vec: Vec<_> = self.lang_stats.iter().collect();
		stats_vec.sort_by_key(|(_, lang_stats)| Reverse(lang_stats.lines));
		stats_vec
	}

	pub(crate) fn code_percentage(&self) -> f64 {
		utils::percentage(self.total_code_lines, self.total_lines)
	}

	pub(crate) fn comment_percentage(&self) -> f64 {
		utils::percentage(self.total_comment_lines, self.total_lines)
	}

	pub(crate) fn blank_percentage(&self) -> f64 {
		utils::percentage(self.total_blank_lines, self.total_lines)
	}
}
