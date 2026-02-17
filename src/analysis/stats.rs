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

	/// Calculate the average lines per file for this language
	#[must_use]
	#[expect(clippy::cast_precision_loss)]
	pub fn average_lines_per_file(&self) -> f64 {
		if self.files == 0 { 0.0 } else { self.lines as f64 / self.files as f64 }
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
	skipped_entries: u64,
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
		self.skipped_entries = self.skipped_entries.saturating_add(other.skipped_entries);
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

	/// Get the number of entries skipped due to errors.
	#[must_use]
	pub const fn skipped_entries(&self) -> u64 {
		self.skipped_entries
	}

	pub(crate) const fn set_skipped_entries(&mut self, skipped: u64) {
		self.skipped_entries = skipped;
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
	use rstest::rstest;

	use super::*;

	#[rstest]
	#[case(0, 100, 0.0, f64::EPSILON)]
	#[case(50, 100, 50.0, f64::EPSILON)]
	#[case(25, 100, 25.0, f64::EPSILON)]
	#[case(100, 100, 100.0, f64::EPSILON)]
	#[case(10, 0, 0.0, f64::EPSILON)]
	#[case(u64::MAX / 2, u64::MAX, 50.0, 0.000_000_1)]
	fn test_percentage(#[case] part: u64, #[case] total: u64, #[case] expected: f64, #[case] epsilon: f64) {
		assert!((percentage(part, total) - expected).abs() <= epsilon);
	}

	#[test]
	fn test_line_stats_merge() {
		let mut a = LineStats::new(10, 5, 3, 1);
		let b = LineStats::new(20, 10, 6, 2);
		a.merge(&b);
		assert_eq!(a.code, 30);
		assert_eq!(a.comment, 15);
		assert_eq!(a.blank, 9);
		assert_eq!(a.shebang, 3);
	}

	#[test]
	fn test_line_stats_merge_saturating() {
		let mut a = LineStats::new(u64::MAX - 1, 0, 0, 0);
		let b = LineStats::new(10, 0, 0, 0);
		a.merge(&b);
		assert_eq!(a.code, u64::MAX);
	}

	#[test]
	fn test_file_contribution() {
		let fc = FileContribution::new(100, 60, 20, 15, 5, 1024);
		assert_eq!(fc.total_lines(), 100);
		assert_eq!(fc.size(), 1024);
	}

	#[test]
	fn test_file_stats() {
		let fs = FileStats::new("test.rs".to_string(), 100, 60, 20, 15, 5, 2048);
		assert_eq!(fs.path(), "test.rs");
		assert_eq!(fs.total_lines(), 100);
		assert_eq!(fs.code_lines(), 60);
		assert_eq!(fs.comment_lines(), 20);
		assert_eq!(fs.blank_lines(), 15);
		assert_eq!(fs.shebang_lines(), 5);
		assert_eq!(fs.size(), 2048);
	}

	#[test]
	fn test_language_stats_add_file() {
		let mut ls = LanguageStats::default();
		let fc = FileContribution::new(50, 30, 10, 8, 2, 512);
		ls.add_file(&fc, None);
		assert_eq!(ls.files(), 1);
		assert_eq!(ls.lines(), 50);
		assert_eq!(ls.code_lines(), 30);
		assert_eq!(ls.comment_lines(), 10);
		assert_eq!(ls.blank_lines(), 8);
		assert_eq!(ls.shebang_lines(), 2);
		assert_eq!(ls.size(), 512);
		assert!(ls.files_list().is_empty());
	}

	#[test]
	fn test_language_stats_add_file_with_details() {
		let mut ls = LanguageStats::default();
		let fc = FileContribution::new(50, 30, 10, 8, 2, 512);
		let fs = FileStats::new("foo.rs".to_string(), 50, 30, 10, 8, 2, 512);
		ls.add_file(&fc, Some(fs));
		assert_eq!(ls.files_list().len(), 1);
		assert_eq!(ls.files_list()[0].path(), "foo.rs");
	}

	#[test]
	fn test_language_stats_merge() {
		let mut a = LanguageStats::default();
		let fc1 = FileContribution::new(50, 30, 10, 8, 2, 512);
		a.add_file(&fc1, None);

		let mut b = LanguageStats::default();
		let fc2 = FileContribution::new(100, 60, 20, 16, 4, 1024);
		b.add_file(&fc2, None);

		a.merge(b);
		assert_eq!(a.files(), 2);
		assert_eq!(a.lines(), 150);
		assert_eq!(a.code_lines(), 90);
		assert_eq!(a.size(), 1536);
	}

	#[test]
	fn test_language_stats_average_lines_per_file() {
		let mut ls = LanguageStats::default();
		assert!(ls.average_lines_per_file().abs() < f64::EPSILON);

		let fc1 = FileContribution::new(100, 50, 25, 20, 5, 1000);
		let fc2 = FileContribution::new(200, 100, 50, 40, 10, 2000);
		ls.add_file(&fc1, None);
		ls.add_file(&fc2, None);
		assert!((ls.average_lines_per_file() - 150.0).abs() < f64::EPSILON);
	}

	#[test]
	fn test_language_stats_percentages() {
		let mut ls = LanguageStats::default();
		let fc = FileContribution::new(100, 60, 20, 18, 2, 1000);
		ls.add_file(&fc, None);
		assert!((ls.code_percentage() - 60.0).abs() < f64::EPSILON);
		assert!((ls.comment_percentage() - 20.0).abs() < f64::EPSILON);
		assert!((ls.blank_percentage() - 18.0).abs() < f64::EPSILON);
		assert!((ls.shebang_percentage() - 2.0).abs() < f64::EPSILON);
	}

	#[test]
	fn test_analysis_results_merge() {
		let mut a = AnalysisResults::with_language_capacity();
		let mut b = AnalysisResults::with_language_capacity();

		// Simulating some skipped entries
		a.set_skipped_entries(2);
		b.set_skipped_entries(3);

		a.merge(b);
		assert_eq!(a.skipped_entries(), 5);
	}

	#[test]
	fn test_analysis_results_totals() {
		let results = AnalysisResults::default();
		assert_eq!(results.total_files(), 0);
		assert_eq!(results.total_lines(), 0);
		assert_eq!(results.total_size(), 0);
		assert_eq!(results.total_code_lines(), 0);
		assert_eq!(results.total_comment_lines(), 0);
		assert_eq!(results.total_blank_lines(), 0);
		assert_eq!(results.total_shebang_lines(), 0);
	}
}
