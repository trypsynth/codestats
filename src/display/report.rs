use std::{cmp::Reverse, path::Path};

use serde::Serialize;

use crate::{
	analysis::{AnalysisResults, LanguageStats},
	utils,
};

#[derive(Debug, Serialize)]
pub struct ReportData<'a> {
	pub analysis_path: String,
	pub summary: Summary,
	pub languages: Vec<LanguageRecord<'a>>,
}

impl<'a> ReportData<'a> {
	#[must_use]
	pub fn from_results(results: &'a AnalysisResults, path: &Path, verbose: bool) -> Self {
		let summary = Summary::from_results(results);
		let languages = LanguageRecord::from_results(results, verbose);
		Self { analysis_path: path.display().to_string(), summary, languages }
	}
}

#[derive(Debug, Serialize)]
pub struct Summary {
	pub total_files: u64,
	pub total_lines: u64,
	pub total_code_lines: u64,
	pub total_comment_lines: u64,
	pub total_blank_lines: u64,
	pub total_shebang_lines: u64,
	pub total_size: u64,
	pub total_size_human: String,
	pub code_percentage: f64,
	pub comment_percentage: f64,
	pub blank_percentage: f64,
	pub shebang_percentage: f64,
}

impl Summary {
	#[must_use]
	fn from_results(results: &AnalysisResults) -> Self {
		Self {
			total_files: results.total_files(),
			total_lines: results.total_lines(),
			total_code_lines: results.total_code_lines(),
			total_comment_lines: results.total_comment_lines(),
			total_blank_lines: results.total_blank_lines(),
			total_shebang_lines: results.total_shebang_lines(),
			total_size: results.total_size(),
			total_size_human: results.total_size_human(),
			code_percentage: results.code_percentage(),
			comment_percentage: results.comment_percentage(),
			blank_percentage: results.blank_percentage(),
			shebang_percentage: results.shebang_percentage(),
		}
	}

	#[must_use]
	pub fn percentage_parts(&self) -> Vec<String> {
		let mut parts = Vec::with_capacity(4);
		if self.total_code_lines > 0 {
			parts.push(format!("{:.1}% code", self.code_percentage));
		}
		if self.total_comment_lines > 0 {
			parts.push(format!("{:.1}% comments", self.comment_percentage));
		}
		if self.total_blank_lines > 0 {
			parts.push(format!("{:.1}% blanks", self.blank_percentage));
		}
		if self.total_shebang_lines > 0 {
			parts.push(format!("{:.1}% shebangs", self.shebang_percentage));
		}
		parts
	}

	#[must_use]
	pub fn line_breakdown_parts(&self, pluralize: bool) -> Vec<String> {
		let mut parts = Vec::with_capacity(4);
		if self.total_code_lines > 0 {
			parts.push(if pluralize {
				format!("{} code {}", self.total_code_lines, utils::pluralize(self.total_code_lines, "line", "lines"))
			} else {
				format!("{} code", self.total_code_lines)
			});
		}
		if self.total_comment_lines > 0 {
			parts.push(if pluralize {
				format!(
					"{} comment {}",
					self.total_comment_lines,
					utils::pluralize(self.total_comment_lines, "line", "lines")
				)
			} else {
				format!("{} comments", self.total_comment_lines)
			});
		}
		if self.total_blank_lines > 0 {
			parts.push(if pluralize {
				format!(
					"{} blank {}",
					self.total_blank_lines,
					utils::pluralize(self.total_blank_lines, "line", "lines")
				)
			} else {
				format!("{} blanks", self.total_blank_lines)
			});
		}
		if self.total_shebang_lines > 0 {
			parts.push(if pluralize {
				format!(
					"{} shebang {}",
					self.total_shebang_lines,
					utils::pluralize(self.total_shebang_lines, "line", "lines")
				)
			} else {
				format!("{} shebangs", self.total_shebang_lines)
			});
		}
		parts
	}
}

#[derive(Debug, Serialize)]
pub struct LanguageRecord<'a> {
	pub name: &'a str,
	pub files: u64,
	pub lines: u64,
	pub code_lines: u64,
	pub comment_lines: u64,
	pub blank_lines: u64,
	pub shebang_lines: u64,
	pub size: u64,
	pub size_human: String,
	pub code_percentage: f64,
	pub comment_percentage: f64,
	pub blank_percentage: f64,
	pub shebang_percentage: f64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub files_detail: Option<Vec<FileRecord<'a>>>,
}

impl<'a> LanguageRecord<'a> {
	#[must_use]
	fn from_results(results: &'a AnalysisResults, verbose: bool) -> Vec<Self> {
		results.languages_by_lines().into_iter().map(|(name, stats)| Self::from_stats(name, stats, verbose)).collect()
	}

	#[must_use]
	fn from_stats(name: &'a str, stats: &'a LanguageStats, verbose: bool) -> Self {
		let files_detail = verbose.then(|| {
			let mut files: Vec<_> = stats.files_list().iter().collect();
			files.sort_by_key(|file| Reverse(file.total_lines()));
			files
				.into_iter()
				.map(|file| FileRecord {
					path: file.path(),
					total_lines: file.total_lines(),
					code_lines: file.code_lines(),
					comment_lines: file.comment_lines(),
					blank_lines: file.blank_lines(),
					shebang_lines: file.shebang_lines(),
					size: file.size(),
					size_human: file.size_human(),
				})
				.collect()
		});
		Self {
			name,
			files: stats.files(),
			lines: stats.lines(),
			code_lines: stats.code_lines(),
			comment_lines: stats.comment_lines(),
			blank_lines: stats.blank_lines(),
			shebang_lines: stats.shebang_lines(),
			size: stats.size(),
			size_human: stats.size_human(),
			code_percentage: stats.code_percentage(),
			comment_percentage: stats.comment_percentage(),
			blank_percentage: stats.blank_percentage(),
			shebang_percentage: stats.shebang_percentage(),
			files_detail,
		}
	}
}

#[derive(Debug, Serialize)]
pub struct FileRecord<'a> {
	pub path: &'a str,
	pub total_lines: u64,
	pub code_lines: u64,
	pub comment_lines: u64,
	pub blank_lines: u64,
	pub shebang_lines: u64,
	pub size: u64,
	pub size_human: String,
}

#[derive(Debug, Serialize)]
pub struct FormattedLanguage<'a> {
	pub name: &'a str,
	pub files: u64,
	pub lines: u64,
	pub code_lines: u64,
	pub comment_lines: u64,
	pub blank_lines: u64,
	pub shebang_lines: u64,
	pub size: u64,
	pub size_human: &'a str,
	pub code_percentage: String,
	pub comment_percentage: String,
	pub blank_percentage: String,
	pub shebang_percentage: String,
	pub files_detail: Option<Vec<FormattedFile<'a>>>,
}

#[derive(Debug, Serialize)]
pub struct FormattedFile<'a> {
	pub path: &'a str,
	pub total_lines: u64,
	pub code_lines: u64,
	pub comment_lines: u64,
	pub blank_lines: u64,
	pub shebang_lines: u64,
	pub size: u64,
	pub size_human: &'a str,
}

impl<'a> ReportData<'a> {
	#[must_use]
	pub fn formatted_languages(&'a self) -> Vec<FormattedLanguage<'a>> {
		self.languages
			.iter()
			.map(|lang| FormattedLanguage {
				name: lang.name,
				files: lang.files,
				lines: lang.lines,
				code_lines: lang.code_lines,
				comment_lines: lang.comment_lines,
				blank_lines: lang.blank_lines,
				shebang_lines: lang.shebang_lines,
				size: lang.size,
				size_human: &lang.size_human,
				code_percentage: format!("{:.1}", lang.code_percentage),
				comment_percentage: format!("{:.1}", lang.comment_percentage),
				blank_percentage: format!("{:.1}", lang.blank_percentage),
				shebang_percentage: format!("{:.1}", lang.shebang_percentage),
				files_detail: lang.files_detail.as_ref().map(|files| {
					files
						.iter()
						.map(|file| FormattedFile {
							path: file.path,
							total_lines: file.total_lines,
							code_lines: file.code_lines,
							comment_lines: file.comment_lines,
							blank_lines: file.blank_lines,
							shebang_lines: file.shebang_lines,
							size: file.size,
							size_human: &file.size_human,
						})
						.collect()
				}),
			})
			.collect()
	}
}
