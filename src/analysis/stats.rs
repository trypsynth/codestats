use std::cmp::Reverse;

use serde::Serialize;

use crate::{langs, utils};

macro_rules! getter {
	($name:ident, $type:ty) => {
		#[must_use]
		pub const fn $name(&self) -> $type {
			self.$name
		}
	};
}

macro_rules! size_human_getter {
	() => {
		#[must_use]
		pub fn size_human(&self) -> String {
			utils::human_size(self.size)
		}
	};
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
struct LineStats {
	code: u64,
	comment: u64,
	blank: u64,
	shebang: u64,
}

impl LineStats {
	const fn new(code: u64, comment: u64, blank: u64, shebang: u64) -> Self {
		Self { code, comment, blank, shebang }
	}

	const fn merge(&mut self, other: &Self) {
		self.code += other.code;
		self.comment += other.comment;
		self.blank += other.blank;
		self.shebang += other.shebang;
	}
}

/// Aggregated data about a single file, used for updating totals without always storing per-file detail.
#[derive(Debug, Clone, Copy)]
pub struct FileContribution {
	total_lines: u64,
	line_stats: LineStats,
	size: u64,
}

impl FileContribution {
	#[must_use]
	pub const fn new(
		total_lines: u64,
		code_lines: u64,
		comment_lines: u64,
		blank_lines: u64,
		shebang_lines: u64,
		size: u64,
	) -> Self {
		Self { total_lines, line_stats: LineStats::new(code_lines, comment_lines, blank_lines, shebang_lines), size }
	}

	getter!(total_lines, u64);
	getter!(size, u64);
}

/// Statistics for a single file
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileStats {
	path: String,
	total_lines: u64,
	line_stats: LineStats,
	size: u64,
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
	pub const fn new(
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
			line_stats: LineStats::new(code_lines, comment_lines, blank_lines, shebang_lines),
			size,
		}
	}

	/// Get the file path
	#[must_use]
	pub fn path(&self) -> &str {
		&self.path
	}

	getter!(total_lines, u64);
	getter!(size, u64);
	size_human_getter!();

	#[must_use]
	pub const fn code_lines(&self) -> u64 {
		self.line_stats.code
	}

	#[must_use]
	pub const fn comment_lines(&self) -> u64 {
		self.line_stats.comment
	}

	#[must_use]
	pub const fn blank_lines(&self) -> u64 {
		self.line_stats.blank
	}

	#[must_use]
	pub const fn shebang_lines(&self) -> u64 {
		self.line_stats.shebang
	}
}

/// Holds statistics about a programming language's usage throughout a project.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct LanguageStats {
	files: u64,
	lines: u64,
	line_stats: LineStats,
	size: u64,
	file_list: Vec<FileStats>,
}

impl LanguageStats {
	pub(crate) fn add_file(&mut self, contribution: &FileContribution, file_stats: Option<FileStats>) {
		self.files += 1;
		self.lines += contribution.total_lines();
		self.line_stats.merge(&contribution.line_stats);
		self.size += contribution.size();
		if let Some(stats) = file_stats {
			self.file_list.push(stats);
		}
	}

	pub(crate) fn merge(&mut self, mut other: Self) {
		self.files += other.files;
		self.lines += other.lines;
		self.line_stats.merge(&other.line_stats);
		self.size += other.size;
		self.file_list.append(&mut other.file_list);
	}

	getter!(files, u64);
	getter!(lines, u64);
	getter!(size, u64);
	size_human_getter!();

	/// Get the number of code lines across all files of this language
	#[must_use]
	pub const fn code_lines(&self) -> u64 {
		self.line_stats.code
	}

	/// Get the number of comment lines across all files of this language
	#[must_use]
	pub const fn comment_lines(&self) -> u64 {
		self.line_stats.comment
	}

	/// Get the number of blank lines across all files of this language
	#[must_use]
	pub const fn blank_lines(&self) -> u64 {
		self.line_stats.blank
	}

	/// Get the number of shebang lines across all files of this language
	#[must_use]
	pub const fn shebang_lines(&self) -> u64 {
		self.line_stats.shebang
	}

	/// Get the list of individual file statistics for this language
	#[must_use]
	pub fn files_list(&self) -> &[FileStats] {
		&self.file_list
	}

	/// Get the percentage of code lines relative to total lines for this language
	#[must_use]
	pub fn code_percentage(&self) -> f64 {
		utils::percentage(self.line_stats.code, self.lines)
	}

	/// Get the percentage of comment lines relative to total lines for this language
	#[must_use]
	pub fn comment_percentage(&self) -> f64 {
		utils::percentage(self.line_stats.comment, self.lines)
	}

	/// Get the percentage of blank lines relative to total lines for this language
	#[must_use]
	pub fn blank_percentage(&self) -> f64 {
		utils::percentage(self.line_stats.blank, self.lines)
	}

	/// Get the percentage of shebang lines relative to total lines for this language
	#[must_use]
	pub fn shebang_percentage(&self) -> f64 {
		utils::percentage(self.line_stats.shebang, self.lines)
	}
}

/// Results of a code analysis operation
#[derive(Debug, Serialize)]
pub struct AnalysisResults {
	total_files: u64,
	total_lines: u64,
	line_stats: LineStats,
	total_size: u64,
	language_stats: Vec<LanguageStats>,
}

impl Default for AnalysisResults {
	fn default() -> Self {
		Self {
			total_files: 0,
			total_lines: 0,
			line_stats: LineStats::default(),
			total_size: 0,
			language_stats: vec![LanguageStats::default(); langs::LANGUAGES.len()],
		}
	}
}

impl AnalysisResults {
	pub(crate) fn add_file_stats(
		&mut self,
		language: &'static langs::Language,
		contribution: FileContribution,
		file_stats: Option<FileStats>,
	) {
		self.total_files += 1;
		self.total_lines += contribution.total_lines();
		self.line_stats.merge(&contribution.line_stats);
		self.total_size += contribution.size();
		self.language_stats[language.index].add_file(&contribution, file_stats);
	}

	pub(crate) fn merge(&mut self, mut other: Self) {
		self.total_files += other.total_files;
		self.total_lines += other.total_lines;
		self.line_stats.merge(&other.line_stats);
		self.total_size += other.total_size;
		for (idx, stats) in other.language_stats.drain(..).enumerate() {
			self.language_stats[idx].merge(stats);
		}
	}

	getter!(total_files, u64);
	getter!(total_lines, u64);

	/// Get the total size in bytes across all files
	#[must_use]
	pub const fn total_size(&self) -> u64 {
		self.total_size
	}

	/// Get the total size in human-readable format across all files
	#[must_use]
	pub fn total_size_human(&self) -> String {
		utils::human_size(self.total_size)
	}

	/// Get the total number of code lines across all files
	#[must_use]
	pub const fn total_code_lines(&self) -> u64 {
		self.line_stats.code
	}

	/// Get the total number of comment lines across all files
	#[must_use]
	pub const fn total_comment_lines(&self) -> u64 {
		self.line_stats.comment
	}

	/// Get the total number of blank lines across all files
	#[must_use]
	pub const fn total_blank_lines(&self) -> u64 {
		self.line_stats.blank
	}

	/// Get the total number of shebang lines across all files
	#[must_use]
	pub const fn total_shebang_lines(&self) -> u64 {
		self.line_stats.shebang
	}

	/// Get languages sorted by total lines in descending order
	///
	/// Returns a vector of tuples containing (`language_name`, `language_stats`)
	/// sorted by the number of lines in each language, with the language
	/// with the most lines coming first.
	#[must_use]
	pub fn languages_by_lines(&self) -> Vec<(&'static str, &LanguageStats)> {
		let mut stats_vec: Vec<_> = langs::LANGUAGES
			.iter()
			.filter_map(|lang| {
				let stats = &self.language_stats[lang.index];
				(stats.files() > 0).then_some((lang.name, stats))
			})
			.collect();
		stats_vec.sort_by_key(|(_, lang_stats)| Reverse(lang_stats.lines()));
		stats_vec
	}

	/// Get the percentage of code lines relative to total lines
	#[must_use]
	pub fn code_percentage(&self) -> f64 {
		utils::percentage(self.line_stats.code, self.total_lines)
	}

	/// Get the percentage of comment lines relative to total lines
	#[must_use]
	pub fn comment_percentage(&self) -> f64 {
		utils::percentage(self.line_stats.comment, self.total_lines)
	}

	/// Get the percentage of blank lines relative to total lines
	#[must_use]
	pub fn blank_percentage(&self) -> f64 {
		utils::percentage(self.line_stats.blank, self.total_lines)
	}

	/// Get the percentage of shebang lines relative to total lines
	#[must_use]
	pub fn shebang_percentage(&self) -> f64 {
		utils::percentage(self.line_stats.shebang, self.total_lines)
	}
}
