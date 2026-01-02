use std::path::Path;

use serde::Serialize;

use crate::{
	analysis::{AnalysisResults, FileStats, LanguageStats, LineType},
	display::{
		apply_sort,
		formatting::{FormatterContext, SortValue, pluralize as pluralize_fn},
		options::LanguageSortKey,
	},
};

macro_rules! impl_formatters {
	($type:ty {
		$($method_name:ident => $field:ident : $formatter:ident),* $(,)?
	}) => {
		impl $type {
			$(
				#[must_use]
				pub fn $method_name(&self, ctx: &FormatterContext) -> String {
					ctx.$formatter(self.$field)
				}
			)*
		}
	};
}

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

#[derive(Debug)]
pub struct SummaryMetric<'a> {
	pub label: &'static str,
	pub value: u64,
	pub percentage: Option<f64>,
	pub human_readable: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct LineTypeStats {
	pub kind: LineType,
	pub count: u64,
	pub percentage: f64,
}

impl LineTypeStats {
	pub const fn singular_label(self) -> &'static str {
		self.kind.singular_label()
	}

	pub const fn plural_label(self) -> &'static str {
		self.kind.plural_label()
	}

	pub const fn title_label(self) -> &'static str {
		self.kind.title_label()
	}
}

#[derive(Clone, Copy)]
struct LineTypeSeries {
	code: u64,
	comment: u64,
	blank: u64,
	shebang: u64,
	code_pct: f64,
	comment_pct: f64,
	blank_pct: f64,
	shebang_pct: f64,
}

fn iter_line_types(series: LineTypeSeries) -> impl Iterator<Item = LineTypeStats> {
	[
		LineTypeStats { kind: LineType::Code, count: series.code, percentage: series.code_pct },
		LineTypeStats { kind: LineType::Comment, count: series.comment, percentage: series.comment_pct },
		LineTypeStats { kind: LineType::Blank, count: series.blank, percentage: series.blank_pct },
		LineTypeStats { kind: LineType::Shebang, count: series.shebang, percentage: series.shebang_pct },
	]
	.into_iter()
	.filter(|info| info.count > 0)
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

	fn iter_line_types(&self) -> impl Iterator<Item = LineTypeStats> + '_ {
		iter_line_types(LineTypeSeries {
			code: self.total_code_lines,
			comment: self.total_comment_lines,
			blank: self.total_blank_lines,
			shebang: self.total_shebang_lines,
			code_pct: self.code_percentage,
			comment_pct: self.comment_percentage,
			blank_pct: self.blank_percentage,
			shebang_pct: self.shebang_percentage,
		})
	}

	#[must_use]
	pub fn percentage_parts(&self, ctx: &FormatterContext) -> Vec<String> {
		self.iter_line_types()
			.map(|info| format!("{}% {}", ctx.percent(info.percentage), info.plural_label()))
			.collect()
	}

	#[must_use]
	pub fn line_breakdown_parts(&self, pluralize: bool, ctx: &FormatterContext) -> Vec<String> {
		self.iter_line_types()
			.map(|info| {
				let formatted = ctx.number(info.count);
				if pluralize {
					format!("{formatted} {} {}", info.singular_label(), pluralize_fn(info.count, "line", "lines"))
				} else {
					format!("{formatted} {}", info.plural_label())
				}
			})
			.collect()
	}

	pub fn metrics(&self) -> impl Iterator<Item = SummaryMetric<'_>> {
		let size_human = self.total_size_human.as_str();
		[
			SummaryMetric { label: "Total Files", value: self.total_files, percentage: None, human_readable: None },
			SummaryMetric { label: "Total Lines", value: self.total_lines, percentage: None, human_readable: None },
			SummaryMetric {
				label: "Code Lines",
				value: self.total_code_lines,
				percentage: Some(self.code_percentage),
				human_readable: None,
			},
			SummaryMetric {
				label: "Comment Lines",
				value: self.total_comment_lines,
				percentage: Some(self.comment_percentage),
				human_readable: None,
			},
			SummaryMetric {
				label: "Blank Lines",
				value: self.total_blank_lines,
				percentage: Some(self.blank_percentage),
				human_readable: None,
			},
			SummaryMetric {
				label: "Shebang Lines",
				value: self.total_shebang_lines,
				percentage: Some(self.shebang_percentage),
				human_readable: None,
			},
			SummaryMetric {
				label: "Total Size",
				value: self.total_size,
				percentage: None,
				human_readable: Some(size_human),
			},
		]
		.into_iter()
	}
}

fn sort_key_for_file_record(file: &FileStats, key: LanguageSortKey) -> SortValue<'_> {
	match key {
		LanguageSortKey::Lines => SortValue::Num(file.total_lines()),
		LanguageSortKey::Code => SortValue::Num(file.code_lines()),
		LanguageSortKey::Comments => SortValue::Num(file.comment_lines()),
		LanguageSortKey::Blanks => SortValue::Num(file.blank_lines()),
		LanguageSortKey::Size => SortValue::Num(file.size()),
		// "Files" is a language-level metric, so fall back to a stable per-file key.
		LanguageSortKey::Files | LanguageSortKey::Name => SortValue::Text(file.path()),
	}
}

const fn sort_key_for_language_record<'a>(
	name: &'a str,
	stats: &'a LanguageStats,
	key: LanguageSortKey,
) -> SortValue<'a> {
	match key {
		LanguageSortKey::Lines => SortValue::Num(stats.lines()),
		LanguageSortKey::Code => SortValue::Num(stats.code_lines()),
		LanguageSortKey::Comments => SortValue::Num(stats.comment_lines()),
		LanguageSortKey::Blanks => SortValue::Num(stats.blank_lines()),
		LanguageSortKey::Files => SortValue::Num(stats.files()),
		LanguageSortKey::Size => SortValue::Num(stats.size()),
		LanguageSortKey::Name => SortValue::Text(name),
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
		let sort_key = ctx.options.language_sort_key;
		apply_sort(
			&mut stats_vec,
			ctx.options.sort_direction,
			|(name, stats)| sort_key_for_language_record(name, stats, sort_key),
			|a, b| a.0.cmp(b.0),
		);
		stats_vec.into_iter().map(|(name, stats)| Self::from_stats(name, stats, verbose, ctx)).collect()
	}

	#[must_use]
	fn from_stats(name: &'a str, stats: &'a LanguageStats, verbose: bool, ctx: &FormatterContext) -> Self {
		let files_detail = verbose.then(|| {
			let mut files: Vec<_> = stats.files_list().iter().collect();
			let sort_key = ctx.options.language_sort_key;
			apply_sort(
				&mut files,
				ctx.options.sort_direction,
				|file| sort_key_for_file_record(file, sort_key),
				|a, b| a.path().cmp(b.path()),
			);
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

	pub fn line_types(&self) -> impl Iterator<Item = LineTypeStats> + '_ {
		iter_line_types(LineTypeSeries {
			code: self.code_lines,
			comment: self.comment_lines,
			blank: self.blank_lines,
			shebang: self.shebang_lines,
			code_pct: self.code_percentage,
			comment_pct: self.comment_percentage,
			blank_pct: self.blank_percentage,
			shebang_pct: self.shebang_percentage,
		})
	}
}

impl_formatters!(LanguageRecord<'_> {
	format_files => files : number,
	format_lines => lines : number,
	format_code_lines => code_lines : number,
	format_comment_lines => comment_lines : number,
	format_blank_lines => blank_lines : number,
	format_shebang_lines => shebang_lines : number,
	format_size => size : number,
	format_code_percentage => code_percentage : percent,
	format_comment_percentage => comment_percentage : percent,
	format_blank_percentage => blank_percentage : percent,
	format_shebang_percentage => shebang_percentage : percent,
});

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

impl_formatters!(FileRecord<'_> {
	format_total_lines => total_lines : number,
	format_code_lines => code_lines : number,
	format_comment_lines => comment_lines : number,
	format_blank_lines => blank_lines : number,
	format_shebang_lines => shebang_lines : number,
	format_size => size : number,
});
