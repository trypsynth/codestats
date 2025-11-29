use std::path::Path;

use serde::Serialize;

use crate::{
	analysis::{AnalysisResults, LanguageStats},
	display::{
		apply_sort,
		formatting::{FormatterContext, SortValue},
		options::LanguageSortKey,
	},
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
	pub fn from_results(results: &'a AnalysisResults, path: &Path, verbose: bool, ctx: &FormatterContext) -> Self {
		let summary = Summary::from_results(results, ctx);
		let languages = LanguageRecord::from_results(results, verbose, ctx);
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
	fn from_results(results: &AnalysisResults, ctx: &FormatterContext) -> Self {
		Self {
			total_files: results.total_files(),
			total_lines: results.total_lines(),
			total_code_lines: results.total_code_lines(),
			total_comment_lines: results.total_comment_lines(),
			total_blank_lines: results.total_blank_lines(),
			total_shebang_lines: results.total_shebang_lines(),
			total_size: results.total_size(),
			total_size_human: ctx.size(results.total_size()),
			code_percentage: results.code_percentage(),
			comment_percentage: results.comment_percentage(),
			blank_percentage: results.blank_percentage(),
			shebang_percentage: results.shebang_percentage(),
		}
	}

	#[must_use]
	pub fn percentage_parts(&self, ctx: &FormatterContext) -> Vec<String> {
		let mut parts = Vec::with_capacity(4);
		if self.total_code_lines > 0 {
			let pct = ctx.percent(self.code_percentage);
			parts.push(format!("{pct}% code"));
		}
		if self.total_comment_lines > 0 {
			let pct = ctx.percent(self.comment_percentage);
			parts.push(format!("{pct}% comments"));
		}
		if self.total_blank_lines > 0 {
			let pct = ctx.percent(self.blank_percentage);
			parts.push(format!("{pct}% blanks"));
		}
		if self.total_shebang_lines > 0 {
			let pct = ctx.percent(self.shebang_percentage);
			parts.push(format!("{pct}% shebangs"));
		}
		parts
	}

	#[must_use]
	pub fn line_breakdown_parts(&self, pluralize: bool, ctx: &FormatterContext) -> Vec<String> {
		let mut parts = Vec::with_capacity(4);
		if self.total_code_lines > 0 {
			let code_lines = ctx.number(self.total_code_lines);
			parts.push(if pluralize {
				format!("{code_lines} code {}", utils::pluralize(self.total_code_lines, "line", "lines"))
			} else {
				format!("{code_lines} code")
			});
		}
		if self.total_comment_lines > 0 {
			let comment_lines = ctx.number(self.total_comment_lines);
			parts.push(if pluralize {
				format!("{comment_lines} comment {}", utils::pluralize(self.total_comment_lines, "line", "lines"))
			} else {
				format!("{comment_lines} comments")
			});
		}
		if self.total_blank_lines > 0 {
			let blank_lines = ctx.number(self.total_blank_lines);
			parts.push(if pluralize {
				format!("{blank_lines} blank {}", utils::pluralize(self.total_blank_lines, "line", "lines"))
			} else {
				format!("{blank_lines} blanks")
			});
		}
		if self.total_shebang_lines > 0 {
			let shebang_lines = ctx.number(self.total_shebang_lines);
			parts.push(if pluralize {
				format!("{shebang_lines} shebang {}", utils::pluralize(self.total_shebang_lines, "line", "lines"))
			} else {
				format!("{shebang_lines} shebangs")
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
	fn from_results(results: &'a AnalysisResults, verbose: bool, ctx: &FormatterContext) -> Vec<Self> {
		let mut stats_vec: Vec<_> = results.languages().map(|(lang, stats)| (lang.name, stats)).collect();
		stats_vec =
			apply_sort(stats_vec, ctx.options.language_sort_key, ctx.options.sort_direction, |(name, stats)| match ctx
				.options
				.language_sort_key
			{
				LanguageSortKey::Lines => SortValue::Num(stats.lines()),
				LanguageSortKey::Code => SortValue::Num(stats.code_lines()),
				LanguageSortKey::Comments => SortValue::Num(stats.comment_lines()),
				LanguageSortKey::Blanks => SortValue::Num(stats.blank_lines()),
				LanguageSortKey::Files => SortValue::Num(stats.files()),
				LanguageSortKey::Size => SortValue::Num(stats.size()),
				LanguageSortKey::Name => SortValue::Text(name),
			});
		stats_vec.into_iter().map(|(name, stats)| Self::from_stats(name, stats, verbose, ctx)).collect()
	}

	#[must_use]
	fn from_stats(name: &'a str, stats: &'a LanguageStats, verbose: bool, ctx: &FormatterContext) -> Self {
		let files_detail = verbose.then(|| {
			let mut files: Vec<_> = stats.files_list().iter().collect();
			files = apply_sort(files, ctx.options.language_sort_key, ctx.options.sort_direction, |file| {
				match ctx.options.language_sort_key {
					LanguageSortKey::Lines | LanguageSortKey::Files => SortValue::Num(file.total_lines()),
					LanguageSortKey::Code => SortValue::Num(file.code_lines()),
					LanguageSortKey::Comments => SortValue::Num(file.comment_lines()),
					LanguageSortKey::Blanks => SortValue::Num(file.blank_lines()),
					LanguageSortKey::Size => SortValue::Num(file.size()),
					LanguageSortKey::Name => SortValue::Text(file.path()),
				}
			});
			files
				.into_iter()
				.map(|file| {
					let size_human = ctx.size(file.size());
					FileRecord {
						path: file.path(),
						total_lines: file.total_lines(),
						code_lines: file.code_lines(),
						comment_lines: file.comment_lines(),
						blank_lines: file.blank_lines(),
						shebang_lines: file.shebang_lines(),
						size: file.size(),
						size_human,
					}
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
			size_human: ctx.size(stats.size()),
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
	pub files: String,
	pub lines: String,
	pub code_lines: String,
	pub comment_lines: String,
	pub blank_lines: String,
	pub shebang_lines: String,
	pub size: String,
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
	pub total_lines: String,
	pub code_lines: String,
	pub comment_lines: String,
	pub blank_lines: String,
	pub shebang_lines: String,
	pub size: String,
	pub size_human: &'a str,
}

impl<'a> ReportData<'a> {
	#[must_use]
	pub fn formatted_languages(&'a self, ctx: &FormatterContext) -> Vec<FormattedLanguage<'a>> {
		self.languages
			.iter()
			.map(|lang| FormattedLanguage {
				name: lang.name,
				files: ctx.number(lang.files),
				lines: ctx.number(lang.lines),
				code_lines: ctx.number(lang.code_lines),
				comment_lines: ctx.number(lang.comment_lines),
				blank_lines: ctx.number(lang.blank_lines),
				shebang_lines: ctx.number(lang.shebang_lines),
				size: ctx.number(lang.size),
				size_human: &lang.size_human,
				code_percentage: ctx.percent(lang.code_percentage),
				comment_percentage: ctx.percent(lang.comment_percentage),
				blank_percentage: ctx.percent(lang.blank_percentage),
				shebang_percentage: ctx.percent(lang.shebang_percentage),
				files_detail: lang.files_detail.as_ref().map(|files| {
					files
						.iter()
						.map(|file| FormattedFile {
							path: file.path,
							total_lines: ctx.number(file.total_lines),
							code_lines: ctx.number(file.code_lines),
							comment_lines: ctx.number(file.comment_lines),
							blank_lines: ctx.number(file.blank_lines),
							shebang_lines: ctx.number(file.shebang_lines),
							size: ctx.number(file.size),
							size_human: &file.size_human,
						})
						.collect()
				}),
			})
			.collect()
	}
}
