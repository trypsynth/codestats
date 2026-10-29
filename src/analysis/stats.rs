use super::LineType;
use crate::langs::{LANGUAGES, Language};

/// Initial capacity for per-language file detail lists in verbose mode. Pre-allocating 256 slots reduces reallocations for most projects while avoiding excessive memory waste for languages with few files.
const INITIAL_FILE_LIST_CAPACITY: usize = 256;

/// Calculate the percentage that `part` represents of `total`.
///
/// Returns `0.0` when `total` is `0` to avoid division-by-zero panics.
#[inline]
#[must_use]
#[expect(
	clippy::cast_precision_loss,
	reason = "percentage values are display-only; sub-percent precision loss is acceptable"
)]
pub fn percentage(part: u64, total: u64) -> f64 {
	if total == 0 { 0.0 } else { (part as f64 / total as f64) * 100.0 }
}

/// Line totals for a file or group of files. `total` is the sum of the other four.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LineCounts {
	pub total: u64,
	pub code: u64,
	pub comment: u64,
	pub blank: u64,
	pub shebang: u64,
}

impl LineCounts {
	pub(crate) const fn add(&mut self, line_type: LineType) {
		match line_type {
			LineType::Code => self.code += 1,
			LineType::Comment => self.comment += 1,
			LineType::Blank => self.blank += 1,
			LineType::Shebang => self.shebang += 1,
		}
		self.total += 1;
	}

	pub(crate) const fn merge(&mut self, other: &Self) {
		self.total = self.total.saturating_add(other.total);
		self.code = self.code.saturating_add(other.code);
		self.comment = self.comment.saturating_add(other.comment);
		self.blank = self.blank.saturating_add(other.blank);
		self.shebang = self.shebang.saturating_add(other.shebang);
	}

	#[must_use]
	pub const fn get(&self, kind: LineType) -> u64 {
		match kind {
			LineType::Code => self.code,
			LineType::Comment => self.comment,
			LineType::Blank => self.blank,
			LineType::Shebang => self.shebang,
		}
	}

	#[must_use]
	pub fn percentage(&self, kind: LineType) -> f64 {
		percentage(self.get(kind), self.total)
	}
}

/// Statistics for a single file
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStats {
	path: String,
	counts: LineCounts,
	size: u64,
}

impl FileStats {
	#[must_use]
	pub fn path(&self) -> &str {
		&self.path
	}

	#[must_use]
	pub const fn counts(&self) -> &LineCounts {
		&self.counts
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
	counts: LineCounts,
	size: u64,
	file_list: Vec<FileStats>,
}

impl LanguageStats {
	fn add_file(&mut self, counts: &LineCounts, size: u64, path: Option<String>) {
		self.files = self.files.saturating_add(1);
		self.counts.merge(counts);
		self.size = self.size.saturating_add(size);
		if let Some(path) = path {
			// Reserve capacity on first file to reduce reallocations
			if self.file_list.is_empty() {
				self.file_list.reserve(INITIAL_FILE_LIST_CAPACITY);
			}
			self.file_list.push(FileStats { path, counts: *counts, size });
		}
	}

	fn merge(&mut self, mut other: Self) {
		self.files = self.files.saturating_add(other.files);
		self.counts.merge(&other.counts);
		self.size = self.size.saturating_add(other.size);
		self.file_list.append(&mut other.file_list);
	}

	#[must_use]
	pub const fn files(&self) -> u64 {
		self.files
	}

	#[must_use]
	pub const fn counts(&self) -> &LineCounts {
		&self.counts
	}

	#[must_use]
	pub const fn size(&self) -> u64 {
		self.size
	}

	/// Get the list of individual file statistics for this language
	#[must_use]
	pub fn files_list(&self) -> &[FileStats] {
		&self.file_list
	}

	/// Calculate the average lines per file for this language
	#[must_use]
	#[expect(
		clippy::cast_precision_loss,
		reason = "average line counts are display-only; sub-unit precision loss is acceptable"
	)]
	pub fn average_lines_per_file(&self) -> f64 {
		if self.files == 0 { 0.0 } else { self.counts.total as f64 / self.files as f64 }
	}
}

/// Results of a code analysis operation
#[derive(Debug, Default)]
pub struct AnalysisResults {
	total_files: u64,
	counts: LineCounts,
	total_size: u64,
	skipped_entries: u64,
	language_stats: Vec<LanguageStats>,
}

impl AnalysisResults {
	#[must_use]
	pub fn with_language_capacity() -> Self {
		Self { language_stats: Vec::with_capacity(LANGUAGES.len()), ..Self::default() }
	}

	/// Record one file. `path` is kept only when per-file details are wanted.
	pub(crate) fn add_file(&mut self, language: &Language, counts: &LineCounts, size: u64, path: Option<String>) {
		if self.language_stats.len() <= language.index {
			self.language_stats.resize_with(language.index + 1, LanguageStats::default);
		}
		self.total_files = self.total_files.saturating_add(1);
		self.counts.merge(counts);
		self.total_size = self.total_size.saturating_add(size);
		self.language_stats[language.index].add_file(counts, size, path);
	}

	pub(crate) fn merge(&mut self, other: Self) {
		self.total_files = self.total_files.saturating_add(other.total_files);
		self.counts.merge(&other.counts);
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
	pub const fn counts(&self) -> &LineCounts {
		&self.counts
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

	/// Iterate over languages that have at least one file, yielding both metadata and stats.
	pub fn languages(&self) -> impl Iterator<Item = (&'static Language, &LanguageStats)> {
		LANGUAGES
			.iter()
			.enumerate()
			.filter_map(|(idx, lang)| self.language_stats.get(idx).map(|stats| (lang, stats)))
			.filter(|(_, stats)| stats.files() > 0)
	}
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;

	const fn counts(total: u64, code: u64, comment: u64, blank: u64, shebang: u64) -> LineCounts {
		LineCounts { total, code, comment, blank, shebang }
	}

	fn rust() -> &'static Language {
		LANGUAGES.iter().find(|l| l.name == "Rust").unwrap()
	}

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
	fn test_line_counts_add() {
		let mut c = LineCounts::default();
		for kind in [LineType::Code, LineType::Code, LineType::Comment, LineType::Blank, LineType::Shebang] {
			c.add(kind);
		}
		assert_eq!(c, counts(5, 2, 1, 1, 1));
	}

	#[test]
	fn test_line_counts_merge() {
		let mut a = counts(19, 10, 5, 3, 1);
		a.merge(&counts(38, 20, 10, 6, 2));
		assert_eq!(a, counts(57, 30, 15, 9, 3));
	}

	#[test]
	fn test_line_counts_merge_saturating() {
		let mut a = counts(0, u64::MAX - 1, 0, 0, 0);
		a.merge(&counts(0, 10, 0, 0, 0));
		assert_eq!(a.code, u64::MAX);
	}

	#[test]
	fn test_line_counts_percentages() {
		let c = counts(100, 60, 20, 18, 2);
		assert!((c.percentage(LineType::Code) - 60.0).abs() < f64::EPSILON);
		assert!((c.percentage(LineType::Comment) - 20.0).abs() < f64::EPSILON);
		assert!((c.percentage(LineType::Blank) - 18.0).abs() < f64::EPSILON);
		assert!((c.percentage(LineType::Shebang) - 2.0).abs() < f64::EPSILON);
	}

	#[test]
	fn test_language_stats_add_file() {
		let mut ls = LanguageStats::default();
		ls.add_file(&counts(50, 30, 10, 8, 2), 512, None);
		assert_eq!(ls.files(), 1);
		assert_eq!(*ls.counts(), counts(50, 30, 10, 8, 2));
		assert_eq!(ls.size(), 512);
		assert_eq!(ls.files_list(), []);
	}

	#[test]
	fn test_language_stats_add_file_with_details() {
		let mut ls = LanguageStats::default();
		ls.add_file(&counts(50, 30, 10, 8, 2), 512, Some("foo.rs".to_owned()));
		assert_eq!(ls.files_list().len(), 1);
		let file = &ls.files_list()[0];
		assert_eq!(file.path(), "foo.rs");
		assert_eq!(*file.counts(), counts(50, 30, 10, 8, 2));
		assert_eq!(file.size(), 512);
	}

	#[test]
	fn test_language_stats_merge() {
		let mut a = LanguageStats::default();
		a.add_file(&counts(50, 30, 10, 8, 2), 512, None);
		let mut b = LanguageStats::default();
		b.add_file(&counts(100, 60, 20, 16, 4), 1024, None);
		a.merge(b);
		assert_eq!(a.files(), 2);
		assert_eq!(a.counts().total, 150);
		assert_eq!(a.counts().code, 90);
		assert_eq!(a.size(), 1536);
	}

	#[test]
	fn test_language_stats_average_lines_per_file() {
		let mut ls = LanguageStats::default();
		assert!(ls.average_lines_per_file().abs() < f64::EPSILON);
		ls.add_file(&counts(100, 50, 25, 20, 5), 1000, None);
		ls.add_file(&counts(200, 100, 50, 40, 10), 2000, None);
		assert!((ls.average_lines_per_file() - 150.0).abs() < f64::EPSILON);
	}

	#[test]
	fn test_analysis_results_add_file() {
		let mut results = AnalysisResults::default();
		results.add_file(rust(), &counts(10, 8, 1, 1, 0), 100, None);
		results.add_file(rust(), &counts(5, 5, 0, 0, 0), 50, None);
		assert_eq!(results.total_files(), 2);
		assert_eq!(*results.counts(), counts(15, 13, 1, 1, 0));
		assert_eq!(results.total_size(), 150);
		let (lang, stats) = results.languages().next().unwrap();
		assert_eq!(lang.name, "Rust");
		assert_eq!(stats.files(), 2);
	}

	#[test]
	fn test_analysis_results_merge() {
		let mut a = AnalysisResults::with_language_capacity();
		let mut b = AnalysisResults::with_language_capacity();
		a.set_skipped_entries(2);
		b.set_skipped_entries(3);
		b.add_file(rust(), &counts(10, 10, 0, 0, 0), 100, None);
		a.merge(b);
		assert_eq!(a.skipped_entries(), 5);
		assert_eq!(a.total_files(), 1);
		assert_eq!(a.counts().code, 10);
	}

	#[test]
	fn test_analysis_results_totals() {
		let results = AnalysisResults::default();
		assert_eq!(results.total_files(), 0);
		assert_eq!(results.total_size(), 0);
		assert_eq!(*results.counts(), LineCounts::default());
	}
}
