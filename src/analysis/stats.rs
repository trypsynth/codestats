use crate::langs::{LANGUAGES, Language};

/// Initial capacity for per-language file detail lists in verbose mode. Pre-allocating 256 slots reduces reallocations for most projects while avoiding excessive memory waste for languages with few files.
const INITIAL_FILE_LIST_CAPACITY: usize = 256;

/// Calculate the percentage that `part` represents of `total`.
///
/// Returns `0.0` when `total` is `0` to avoid division-by-zero panics.
#[inline]
#[expect(clippy::cast_precision_loss)]
pub fn percentage(part: u64, total: u64) -> f64 {
	if total == 0 { 0.0 } else { (part as f64 / total as f64) * 100.0 }
}

macro_rules! impl_percentage_methods {
	($type:ty, $total_field:ident, $stats_field:ident) => {
		impl $type {
			#[must_use]
			pub fn code_percentage(&self) -> f64 {
				percentage(self.$stats_field.code, self.$total_field)
			}
			#[must_use]
			pub fn comment_percentage(&self) -> f64 {
				percentage(self.$stats_field.comment, self.$total_field)
			}
			#[must_use]
			pub fn blank_percentage(&self) -> f64 {
				percentage(self.$stats_field.blank, self.$total_field)
			}
			#[must_use]
			pub fn shebang_percentage(&self) -> f64 {
				percentage(self.$stats_field.shebang, self.$total_field)
			}
		}
	};
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
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
		self.code = self.code.saturating_add(other.code);
		self.comment = self.comment.saturating_add(other.comment);
		self.blank = self.blank.saturating_add(other.blank);
		self.shebang = self.shebang.saturating_add(other.shebang);
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

	#[must_use]
	pub const fn total_lines(&self) -> u64 {
		self.total_lines
	}

	#[must_use]
	pub const fn size(&self) -> u64 {
		self.size
	}
}

/// Statistics for a single file
#[derive(Debug, Clone, PartialEq, Eq)]
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

	#[must_use]
	pub const fn total_lines(&self) -> u64 {
		self.total_lines
	}

	#[must_use]
	pub const fn size(&self) -> u64 {
		self.size
	}

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
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LanguageStats {
	files: u64,
	lines: u64,
	line_stats: LineStats,
	size: u64,
	file_list: Vec<FileStats>,
}

impl LanguageStats {
	pub(crate) fn add_file(&mut self, contribution: &FileContribution, file_stats: Option<FileStats>) {
		self.files = self.files.saturating_add(1);
		self.lines = self.lines.saturating_add(contribution.total_lines());
		self.line_stats.merge(&contribution.line_stats);
		self.size = self.size.saturating_add(contribution.size());
		if let Some(stats) = file_stats {
			// Reserve capacity on first file to reduce reallocations
			if self.file_list.is_empty() {
				self.file_list.reserve(INITIAL_FILE_LIST_CAPACITY);
			}
			self.file_list.push(stats);
		}
	}

	pub(crate) fn merge(&mut self, mut other: Self) {
		self.files = self.files.saturating_add(other.files);
		self.lines = self.lines.saturating_add(other.lines);
		self.line_stats.merge(&other.line_stats);
		self.size = self.size.saturating_add(other.size);
		self.file_list.append(&mut other.file_list);
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
	pub const fn size(&self) -> u64 {
		self.size
	}

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
}

impl_percentage_methods!(LanguageStats, lines, line_stats);

/// Results of a code analysis operation
#[derive(Debug, Default)]
pub struct AnalysisResults {
	total_files: u64,
	total_lines: u64,
	line_stats: LineStats,
	total_size: u64,
	language_stats: Vec<LanguageStats>,
}

impl AnalysisResults {
	#[must_use]
	pub fn with_language_capacity() -> Self {
		Self { language_stats: Vec::with_capacity(LANGUAGES.len()), ..Self::default() }
	}

	fn ensure_language_slot(&mut self, language: &Language) {
		let target_len = language.index + 1;
		if self.language_stats.len() < target_len {
			self.language_stats.resize_with(target_len, LanguageStats::default);
		}
	}

	pub(crate) fn add_file_stats(
		&mut self,
		language: &'static Language,
		contribution: FileContribution,
		file_stats: Option<FileStats>,
	) {
		self.ensure_language_slot(language);
		self.total_files = self.total_files.saturating_add(1);
		self.total_lines = self.total_lines.saturating_add(contribution.total_lines());
		self.line_stats.merge(&contribution.line_stats);
		self.total_size = self.total_size.saturating_add(contribution.size());
		self.language_stats[language.index].add_file(&contribution, file_stats);
	}

	pub(crate) fn merge(&mut self, other: Self) {
		self.total_files = self.total_files.saturating_add(other.total_files);
		self.total_lines = self.total_lines.saturating_add(other.total_lines);
		self.line_stats.merge(&other.line_stats);
		self.total_size = self.total_size.saturating_add(other.total_size);
		if self.language_stats.len() < other.language_stats.len() {
			self.language_stats.resize_with(other.language_stats.len(), LanguageStats::default);
		}
		for (idx, stats) in other.language_stats.into_iter().enumerate() {
			self.language_stats[idx].merge(stats);
		}
	}

	#[must_use]
	pub const fn total_files(&self) -> u64 {
		self.total_files
	}

	#[must_use]
	pub const fn total_lines(&self) -> u64 {
		self.total_lines
	}

	/// Get the total size in bytes across all files
	#[must_use]
	pub const fn total_size(&self) -> u64 {
		self.total_size
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

	/// Iterate over languages that have at least one file, yielding both metadata and stats.
	pub fn languages(&self) -> impl Iterator<Item = (&'static Language, &LanguageStats)> {
		LANGUAGES
			.iter()
			.enumerate()
			.filter_map(|(idx, lang)| self.language_stats.get(idx).map(|stats| (lang, stats)))
			.filter(|(_, stats)| stats.files() > 0)
	}
}

impl_percentage_methods!(AnalysisResults, total_lines, line_stats);

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_percentage() {
		const EPSILON: f64 = f64::EPSILON;
		assert!((percentage(0, 100) - 0.0).abs() <= EPSILON);
		assert!((percentage(50, 100) - 50.0).abs() <= EPSILON);
		assert!((percentage(25, 100) - 25.0).abs() <= EPSILON);
		assert!((percentage(100, 100) - 100.0).abs() <= EPSILON);
		assert!((percentage(10, 0) - 0.0).abs() <= EPSILON);
		let part = u64::MAX / 2;
		let total = u64::MAX;
		let pct = percentage(part, total);
		assert!((pct - 50.0).abs() < 0.000_000_1);
	}
}
