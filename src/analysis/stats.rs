use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::utils;

/// Statistics for a single file
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileStats {
	path: String,
	total_lines: u64,
	code_lines: u64,
	comment_lines: u64,
	blank_lines: u64,
	shebang_lines: u64,
	size: u64,
	size_human: String,
}

impl FileStats {
	/// Create new file statistics
	///
	/// # Arguments
	///
	/// * `path` - The file path
	/// * `total_lines` - Total number of lines in the file
	/// * `code_lines` - Number of lines containing code
	/// * `comment_lines` - Number of lines containing comments
	/// * `blank_lines` - Number of blank lines
	/// * `shebang_lines` - Number of shebang lines
	/// * `size` - File size in bytes
	#[must_use]
	pub fn new(
		path: String,
		total_lines: u64,
		code_lines: u64,
		comment_lines: u64,
		blank_lines: u64,
		shebang_lines: u64,
		size: u64,
	) -> Self {
		Self {
			path,
			total_lines,
			code_lines,
			comment_lines,
			blank_lines,
			shebang_lines,
			size,
			size_human: utils::human_size(size),
		}
	}

	/// Get the file path
	#[must_use]
	pub fn path(&self) -> &str {
		&self.path
	}

	/// Get the total number of lines in the file
	#[must_use]
	pub const fn total_lines(&self) -> u64 {
		self.total_lines
	}

	/// Get the number of lines containing code
	#[must_use]
	pub const fn code_lines(&self) -> u64 {
		self.code_lines
	}

	/// Get the number of lines containing comments
	#[must_use]
	pub const fn comment_lines(&self) -> u64 {
		self.comment_lines
	}

	/// Get the number of blank lines
	#[must_use]
	pub const fn blank_lines(&self) -> u64 {
		self.blank_lines
	}

	/// Get the number of shebang lines
	#[must_use]
	pub const fn shebang_lines(&self) -> u64 {
		self.shebang_lines
	}

	/// Get the file size in bytes
	#[must_use]
	pub const fn size(&self) -> u64 {
		self.size
	}

	/// Get the file size in human-readable format
	#[must_use]
	pub fn size_human(&self) -> &str {
		&self.size_human
	}
}

/// Holds statistics about a programming language's usage throughout a project.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageStats {
	files: u64,
	lines: u64,
	code_lines: u64,
	comment_lines: u64,
	blank_lines: u64,
	shebang_lines: u64,
	size: u64,
	size_human: String,
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

	pub(crate) fn finalize(&mut self) {
		self.size_human = utils::human_size(self.size);
	}

	/// Get the number of files of this language
	#[must_use]
	pub const fn files(&self) -> u64 {
		self.files
	}

	/// Get the total number of lines across all files of this language
	#[must_use]
	pub const fn lines(&self) -> u64 {
		self.lines
	}

	/// Get the number of code lines across all files of this language
	#[must_use]
	pub const fn code_lines(&self) -> u64 {
		self.code_lines
	}

	/// Get the number of comment lines across all files of this language
	#[must_use]
	pub const fn comment_lines(&self) -> u64 {
		self.comment_lines
	}

	/// Get the number of blank lines across all files of this language
	#[must_use]
	pub const fn blank_lines(&self) -> u64 {
		self.blank_lines
	}

	/// Get the number of shebang lines across all files of this language
	#[must_use]
	pub const fn shebang_lines(&self) -> u64 {
		self.shebang_lines
	}

	/// Get the total size in bytes across all files of this language
	#[must_use]
	pub const fn size(&self) -> u64 {
		self.size
	}

	/// Get the total size in human-readable format across all files of this language
	#[must_use]
	pub fn size_human(&self) -> &str {
		&self.size_human
	}

	/// Get the list of individual file statistics for this language
	#[must_use]
	pub fn files_list(&self) -> &[FileStats] {
		&self.file_list
	}

	/// Get the percentage of code lines relative to total lines for this language
	#[must_use]
	pub fn code_percentage(&self) -> f64 {
		utils::percentage(self.code_lines, self.lines)
	}

	/// Get the percentage of comment lines relative to total lines for this language
	#[must_use]
	pub fn comment_percentage(&self) -> f64 {
		utils::percentage(self.comment_lines, self.lines)
	}

	/// Get the percentage of blank lines relative to total lines for this language
	#[must_use]
	pub fn blank_percentage(&self) -> f64 {
		utils::percentage(self.blank_lines, self.lines)
	}

	/// Get the percentage of shebang lines relative to total lines for this language
	#[must_use]
	pub fn shebang_percentage(&self) -> f64 {
		utils::percentage(self.shebang_lines, self.lines)
	}
}

/// Results of a code analysis operation
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AnalysisResults {
	total_files: u64,
	total_lines: u64,
	total_code_lines: u64,
	total_comment_lines: u64,
	total_blank_lines: u64,
	total_shebang_lines: u64,
	total_size: u64,
	total_size_human: String,
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

	/// Get the total number of files analyzed
	#[must_use]
	pub const fn total_files(&self) -> u64 {
		self.total_files
	}

	/// Get the total number of lines across all files
	#[must_use]
	pub const fn total_lines(&self) -> u64 {
		self.total_lines
	}

	/// Get the total number of code lines across all files
	#[must_use]
	pub const fn total_code_lines(&self) -> u64 {
		self.total_code_lines
	}

	/// Get the total number of comment lines across all files
	#[must_use]
	pub const fn total_comment_lines(&self) -> u64 {
		self.total_comment_lines
	}

	/// Get the total number of blank lines across all files
	#[must_use]
	pub const fn total_blank_lines(&self) -> u64 {
		self.total_blank_lines
	}

	/// Get the total number of shebang lines across all files
	#[must_use]
	pub const fn total_shebang_lines(&self) -> u64 {
		self.total_shebang_lines
	}

	/// Get the total size in bytes across all files
	#[must_use]
	pub const fn total_size(&self) -> u64 {
		self.total_size
	}

	/// Get the total size in human-readable format across all files
	#[must_use]
	pub fn total_size_human(&self) -> &str {
		&self.total_size_human
	}

	/// Get a map of all language statistics
	#[must_use]
	pub const fn language_stats(&self) -> &HashMap<String, LanguageStats> {
		&self.language_stats
	}

	/// Get languages sorted by total lines in descending order
	///
	/// Returns a vector of tuples containing (`language_name`, `language_stats`)
	/// sorted by the number of lines in each language, with the language
	/// with the most lines coming first.
	#[must_use]
	pub fn languages_by_lines(&self) -> Vec<(&String, &LanguageStats)> {
		let mut stats_vec: Vec<_> = self.language_stats.iter().collect();
		stats_vec.sort_by_key(|(_, lang_stats)| std::cmp::Reverse(lang_stats.lines));
		stats_vec
	}

	/// Get the percentage of code lines relative to total lines
	#[must_use]
	pub fn code_percentage(&self) -> f64 {
		utils::percentage(self.total_code_lines, self.total_lines)
	}

	/// Get the percentage of comment lines relative to total lines
	#[must_use]
	pub fn comment_percentage(&self) -> f64 {
		utils::percentage(self.total_comment_lines, self.total_lines)
	}

	/// Get the percentage of blank lines relative to total lines
	#[must_use]
	pub fn blank_percentage(&self) -> f64 {
		utils::percentage(self.total_blank_lines, self.total_lines)
	}

	/// Get the percentage of shebang lines relative to total lines
	#[must_use]
	pub fn shebang_percentage(&self) -> f64 {
		utils::percentage(self.total_shebang_lines, self.total_lines)
	}

	pub(crate) fn finalize(&mut self) {
		self.total_size_human = utils::human_size(self.total_size);
		for lang_stats in self.language_stats.values_mut() {
			lang_stats.finalize();
		}
	}
}
