use std::collections::HashMap;

use crate::utils;

/// Statistics for a single file
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStats {
	path: String,
	total_lines: u64,
	code_lines: u64,
	comment_lines: u64,
	blank_lines: u64,
	shebang_lines: u64,
	size: u64,
}

impl FileStats {
	#[must_use]
	pub const fn new(
		path: String,
		total_lines: u64,
		code_lines: u64,
		comment_lines: u64,
		blank_lines: u64,
		shebang_lines: u64,
		size: u64,
	) -> Self {
		Self { path, total_lines, code_lines, comment_lines, blank_lines, shebang_lines, size }
	}

	#[must_use]
	pub fn path(&self) -> &str {
		&self.path
	}

	#[must_use]
	pub const fn total_lines(&self) -> u64 {
		self.total_lines
	}

	#[must_use]
	pub const fn code_lines(&self) -> u64 {
		self.code_lines
	}

	#[must_use]
	pub const fn comment_lines(&self) -> u64 {
		self.comment_lines
	}

	#[must_use]
	pub const fn blank_lines(&self) -> u64 {
		self.blank_lines
	}

	#[must_use]
	pub const fn shebang_lines(&self) -> u64 {
		self.shebang_lines
	}

	#[must_use]
	pub const fn size(&self) -> u64 {
		self.size
	}
}

/// Holds statistics about a programming language's usage throughout a project.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LanguageStats {
	files: u64,
	lines: u64,
	code_lines: u64,
	comment_lines: u64,
	blank_lines: u64,
	shebang_lines: u64,
	size: u64,
	file_list: Vec<FileStats>,
}

impl LanguageStats {
	pub(crate) fn add_file(&mut self, file_stats: FileStats) {
		self.files += 1;
		self.lines += file_stats.total_lines;
		self.code_lines += file_stats.code_lines;
		self.comment_lines += file_stats.comment_lines;
		self.blank_lines += file_stats.blank_lines;
		self.shebang_lines += file_stats.shebang_lines;
		self.size += file_stats.size;
		self.file_list.push(file_stats);
	}

	#[must_use]
	pub const fn files(&self) -> u64 {
		self.files
	}

	#[must_use]
	pub const fn lines(&self) -> u64 {
		self.lines
	}

	#[must_use]
	pub const fn code_lines(&self) -> u64 {
		self.code_lines
	}

	#[must_use]
	pub const fn comment_lines(&self) -> u64 {
		self.comment_lines
	}

	#[must_use]
	pub const fn blank_lines(&self) -> u64 {
		self.blank_lines
	}

	#[must_use]
	pub const fn shebang_lines(&self) -> u64 {
		self.shebang_lines
	}

	#[must_use]
	pub const fn size(&self) -> u64 {
		self.size
	}

	#[must_use]
	pub fn files_list(&self) -> &[FileStats] {
		&self.file_list
	}

	#[must_use]
	pub fn code_percentage(&self) -> f64 {
		utils::percentage(self.code_lines, self.lines)
	}

	#[must_use]
	pub fn comment_percentage(&self) -> f64 {
		utils::percentage(self.comment_lines, self.lines)
	}

	#[must_use]
	pub fn blank_percentage(&self) -> f64 {
		utils::percentage(self.blank_lines, self.lines)
	}

	#[must_use]
	pub fn shebang_percentage(&self) -> f64 {
		utils::percentage(self.shebang_lines, self.lines)
	}
}

/// Results of a code analysis operation
#[derive(Debug, Default)]
pub struct AnalysisResults {
	total_files: u64,
	total_lines: u64,
	total_code_lines: u64,
	total_comment_lines: u64,
	total_blank_lines: u64,
	total_shebang_lines: u64,
	total_size: u64,
	language_stats: HashMap<String, LanguageStats>,
}

impl AnalysisResults {
	pub(crate) fn add_file_stats(&mut self, language: String, file_stats: FileStats) {
		self.total_files += 1;
		self.total_lines += file_stats.total_lines;
		self.total_code_lines += file_stats.code_lines;
		self.total_comment_lines += file_stats.comment_lines;
		self.total_blank_lines += file_stats.blank_lines;
		self.total_shebang_lines += file_stats.shebang_lines;
		self.total_size += file_stats.size;
		self.language_stats.entry(language).or_default().add_file(file_stats);
	}

	#[must_use]
	pub const fn total_files(&self) -> u64 {
		self.total_files
	}

	#[must_use]
	pub const fn total_lines(&self) -> u64 {
		self.total_lines
	}

	#[must_use]
	pub const fn total_code_lines(&self) -> u64 {
		self.total_code_lines
	}

	#[must_use]
	pub const fn total_comment_lines(&self) -> u64 {
		self.total_comment_lines
	}

	#[must_use]
	pub const fn total_blank_lines(&self) -> u64 {
		self.total_blank_lines
	}

	#[must_use]
	pub const fn total_shebang_lines(&self) -> u64 {
		self.total_shebang_lines
	}

	#[must_use]
	pub const fn total_size(&self) -> u64 {
		self.total_size
	}

	#[must_use]
	pub const fn language_stats(&self) -> &HashMap<String, LanguageStats> {
		&self.language_stats
	}

	#[must_use]
	pub fn languages_by_lines(&self) -> Vec<(&String, &LanguageStats)> {
		let mut stats_vec: Vec<_> = self.language_stats.iter().collect();
		stats_vec.sort_by_key(|(_, lang_stats)| std::cmp::Reverse(lang_stats.lines));
		stats_vec
	}

	#[must_use]
	pub fn code_percentage(&self) -> f64 {
		utils::percentage(self.total_code_lines, self.total_lines)
	}

	#[must_use]
	pub fn comment_percentage(&self) -> f64 {
		utils::percentage(self.total_comment_lines, self.total_lines)
	}

	#[must_use]
	pub fn blank_percentage(&self) -> f64 {
		utils::percentage(self.total_blank_lines, self.total_lines)
	}

	#[must_use]
	pub fn shebang_percentage(&self) -> f64 {
		utils::percentage(self.total_shebang_lines, self.total_lines)
	}
}
