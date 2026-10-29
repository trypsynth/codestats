use std::{
	collections::HashMap,
	path::{Component, Path},
};

use serde::Serialize;

use crate::{
	analysis::{AnalysisResults, FileStats, LanguageStats, LineCounts, LineType},
	display::{
		apply_sort,
		formatting::{FormatterContext, SortValue, pluralize as pluralize_fn},
		options::{LanguageSortKey, Verbosity},
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
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub directories: Vec<DirRecord<'a>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub languages_hidden: Option<usize>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub dirs_hidden: Option<usize>,
}

impl<'a> ReportData<'a> {
	#[must_use]
	pub fn from_results(
		results: &'a AnalysisResults,
		path: &Path,
		verbosity: Verbosity,
		ctx: &FormatterContext,
	) -> Self {
		let summary = Summary::from_results(results, ctx);
		let (languages, languages_hidden) = if verbosity > Verbosity::Summary {
			LanguageRecord::from_results(results, verbosity, ctx)
		} else {
			(Vec::default(), 0)
		};
		let (directories, dirs_hidden) = if ctx.options.by_dir && verbosity > Verbosity::Summary {
			DirRecord::from_results(results, path, ctx)
		} else {
			(Vec::new(), 0)
		};
		let languages_hidden = (languages_hidden > 0).then_some(languages_hidden);
		let dirs_hidden = (dirs_hidden > 0).then_some(dirs_hidden);
		Self {
			analysis_path: path.display().to_string(),
			summary,
			languages,
			directories,
			languages_hidden,
			dirs_hidden,
		}
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

/// The line types present in `counts`, with their share of its total.
fn line_types(counts: LineCounts) -> impl Iterator<Item = LineTypeStats> {
	[LineType::Code, LineType::Comment, LineType::Blank, LineType::Shebang]
		.into_iter()
		.map(move |kind| LineTypeStats { kind, count: counts.get(kind), percentage: counts.percentage(kind) })
		.filter(|info| info.count > 0)
}

impl Summary {
	#[must_use]
	fn from_results(results: &AnalysisResults, ctx: &FormatterContext) -> Self {
		let counts = results.counts();
		Self {
			total_files: results.total_files(),
			total_lines: counts.total,
			total_code_lines: counts.code,
			total_comment_lines: counts.comment,
			total_blank_lines: counts.blank,
			total_shebang_lines: counts.shebang,
			total_size: results.total_size(),
			total_size_human: ctx.size(results.total_size()),
			code_percentage: counts.percentage(LineType::Code),
			comment_percentage: counts.percentage(LineType::Comment),
			blank_percentage: counts.percentage(LineType::Blank),
			shebang_percentage: counts.percentage(LineType::Shebang),
		}
	}

	fn iter_line_types(&self) -> impl Iterator<Item = LineTypeStats> {
		line_types(LineCounts {
			total: self.total_lines,
			code: self.total_code_lines,
			comment: self.total_comment_lines,
			blank: self.total_blank_lines,
			shebang: self.total_shebang_lines,
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

/// The value to order a row by for `key`. Rows without a file count (single files) sort by name for `Files`.
pub(super) const fn sort_value<'a>(
	key: LanguageSortKey,
	name: &'a str,
	counts: &LineCounts,
	files: Option<u64>,
	size: u64,
) -> SortValue<'a> {
	match (key, files) {
		(LanguageSortKey::Lines, _) => SortValue::Num(counts.total),
		(LanguageSortKey::Code, _) => SortValue::Num(counts.code),
		(LanguageSortKey::Comments, _) => SortValue::Num(counts.comment),
		(LanguageSortKey::Blanks, _) => SortValue::Num(counts.blank),
		(LanguageSortKey::Size, _) => SortValue::Num(size),
		(LanguageSortKey::Files, Some(files)) => SortValue::Num(files),
		(LanguageSortKey::Files, None) | (LanguageSortKey::Name, _) => SortValue::Text(name),
	}
}

/// Sort a group's files by the configured key and turn them into records.
fn file_records<'a>(mut files: Vec<&'a FileStats>, ctx: &FormatterContext) -> Vec<FileRecord<'a>> {
	let sort_key = ctx.options.language_sort_key;
	apply_sort(
		&mut files,
		ctx.options.sort_direction,
		|file| sort_value(sort_key, file.path(), file.counts(), None, file.size()),
		|a, b| a.path().cmp(b.path()),
	);
	files.into_iter().map(|file| FileRecord::from_stats(file, ctx)).collect()
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
	pub avg_lines_per_file: f64,
	pub code_percentage: f64,
	pub comment_percentage: f64,
	pub blank_percentage: f64,
	pub shebang_percentage: f64,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub files_detail: Option<Vec<FileRecord<'a>>>,
}

impl<'a> LanguageRecord<'a> {
	#[must_use]
	fn from_results(results: &'a AnalysisResults, verbosity: Verbosity, ctx: &FormatterContext) -> (Vec<Self>, usize) {
		let mut stats_vec: Vec<_> = results.languages().map(|(lang, stats)| (lang.name, stats)).collect();
		let total = stats_vec.len();
		let sort_key = ctx.options.language_sort_key;
		apply_sort(
			&mut stats_vec,
			ctx.options.sort_direction,
			|(name, stats)| sort_value(sort_key, name, stats.counts(), Some(stats.files()), stats.size()),
			|a, b| a.0.cmp(b.0),
		);
		if let Some(n) = ctx.options.top_languages {
			stats_vec.truncate(n);
		}
		if let Some(min) = ctx.options.min_lines {
			stats_vec.retain(|(_, stats)| stats.counts().total >= min);
		}
		let hidden = total.saturating_sub(stats_vec.len());
		let records =
			stats_vec.into_iter().map(|(name, stats)| Self::from_stats(name, stats, verbosity, ctx)).collect();
		(records, hidden)
	}

	#[must_use]
	fn from_stats(name: &'a str, stats: &'a LanguageStats, verbosity: Verbosity, ctx: &FormatterContext) -> Self {
		let files_detail =
			(verbosity == Verbosity::Verbose).then(|| file_records(stats.files_list().iter().collect(), ctx));
		let counts = stats.counts();
		Self {
			name,
			files: stats.files(),
			lines: counts.total,
			code_lines: counts.code,
			comment_lines: counts.comment,
			blank_lines: counts.blank,
			shebang_lines: counts.shebang,
			size: stats.size(),
			size_human: ctx.size(stats.size()),
			avg_lines_per_file: stats.average_lines_per_file(),
			code_percentage: counts.percentage(LineType::Code),
			comment_percentage: counts.percentage(LineType::Comment),
			blank_percentage: counts.percentage(LineType::Blank),
			shebang_percentage: counts.percentage(LineType::Shebang),
			files_detail,
		}
	}

	pub fn line_types(&self) -> impl Iterator<Item = LineTypeStats> {
		line_types(LineCounts {
			total: self.lines,
			code: self.code_lines,
			comment: self.comment_lines,
			blank: self.blank_lines,
			shebang: self.shebang_lines,
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

impl<'a> FileRecord<'a> {
	fn from_stats(file: &'a FileStats, ctx: &FormatterContext) -> Self {
		let counts = file.counts();
		Self {
			path: file.path(),
			total_lines: counts.total,
			code_lines: counts.code,
			comment_lines: counts.comment,
			blank_lines: counts.blank,
			shebang_lines: counts.shebang,
			size: file.size(),
			size_human: ctx.size(file.size()),
		}
	}
}

impl_formatters!(FileRecord<'_> {
	format_total_lines => total_lines : number,
	format_code_lines => code_lines : number,
	format_comment_lines => comment_lines : number,
	format_blank_lines => blank_lines : number,
	format_shebang_lines => shebang_lines : number,
	format_size => size : number,
});

fn dir_key(file_path_str: &str, root: &Path) -> String {
	let path = Path::new(file_path_str);
	let relative = path.strip_prefix(root).unwrap_or(path);
	let mut components = relative.components();
	match (components.next(), components.next()) {
		(Some(Component::Normal(first)), Some(_)) => first.to_string_lossy().into_owned(),
		_ => "(root)".to_owned(),
	}
}

#[derive(Debug, Default)]
struct DirAccumulator<'a> {
	files: Vec<&'a FileStats>,
	counts: LineCounts,
	size: u64,
}

#[derive(Debug, Serialize)]
pub struct DirRecord<'a> {
	pub path: String,
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

impl<'a> DirRecord<'a> {
	fn from_results(results: &'a AnalysisResults, root: &Path, ctx: &FormatterContext) -> (Vec<Self>, usize) {
		let verbose = ctx.options.verbosity == Verbosity::Verbose;
		let mut map: HashMap<String, DirAccumulator<'a>> = HashMap::new();
		for (_, stats) in results.languages() {
			for file in stats.files_list() {
				let acc = map.entry(dir_key(file.path(), root)).or_default();
				acc.files.push(file);
				acc.counts.merge(file.counts());
				acc.size = acc.size.saturating_add(file.size());
			}
		}
		let total = map.len();
		let mut records: Vec<_> = map
			.into_iter()
			.map(|(path, acc)| {
				let counts = acc.counts;
				Self {
					files: acc.files.len() as u64,
					lines: counts.total,
					code_lines: counts.code,
					comment_lines: counts.comment,
					blank_lines: counts.blank,
					shebang_lines: counts.shebang,
					size: acc.size,
					size_human: ctx.size(acc.size),
					code_percentage: counts.percentage(LineType::Code),
					comment_percentage: counts.percentage(LineType::Comment),
					blank_percentage: counts.percentage(LineType::Blank),
					shebang_percentage: counts.percentage(LineType::Shebang),
					files_detail: verbose.then(|| file_records(acc.files, ctx)),
					path,
				}
			})
			.collect();
		let sort_key = ctx.options.language_sort_key;
		apply_sort(
			&mut records,
			ctx.options.sort_direction,
			|r| sort_value(sort_key, &r.path, &r.line_counts(), Some(r.files), r.size),
			|a, b| a.path.cmp(&b.path),
		);
		if let Some(n) = ctx.options.top_languages {
			records.truncate(n);
		}
		if let Some(min) = ctx.options.min_lines {
			records.retain(|r| r.lines >= min);
		}
		let hidden = total.saturating_sub(records.len());
		(records, hidden)
	}

	const fn line_counts(&self) -> LineCounts {
		LineCounts {
			total: self.lines,
			code: self.code_lines,
			comment: self.comment_lines,
			blank: self.blank_lines,
			shebang: self.shebang_lines,
		}
	}

	pub fn line_types(&self) -> impl Iterator<Item = LineTypeStats> {
		line_types(self.line_counts())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::display::{formatting::FormatterContext, options::ViewOptions};

	#[test]
	fn summary_line_breakdown_parts_skip_zero() {
		let summary = Summary {
			total_files: 1,
			total_lines: 10,
			total_code_lines: 5,
			total_comment_lines: 2,
			total_blank_lines: 3,
			total_shebang_lines: 0,
			total_size: 0,
			total_size_human: "0 B".to_string(),
			code_percentage: 50.0,
			comment_percentage: 20.0,
			blank_percentage: 30.0,
			shebang_percentage: 0.0,
		};
		let ctx = FormatterContext::new(ViewOptions::default());
		let parts = summary.line_breakdown_parts(true, &ctx);
		assert_eq!(parts, vec!["5 code lines", "2 comment lines", "3 blank lines"]);
	}

	#[test]
	fn summary_percentage_parts_skip_zero() {
		let summary = Summary {
			total_files: 1,
			total_lines: 10,
			total_code_lines: 5,
			total_comment_lines: 2,
			total_blank_lines: 3,
			total_shebang_lines: 0,
			total_size: 0,
			total_size_human: "0 B".to_string(),
			code_percentage: 50.0,
			comment_percentage: 20.0,
			blank_percentage: 30.0,
			shebang_percentage: 0.0,
		};
		let ctx = FormatterContext::new(ViewOptions::default());
		let parts = summary.percentage_parts(&ctx);
		assert_eq!(parts, vec!["50.0% code", "20.0% comments", "30.0% blanks"]);
	}

	#[test]
	fn summary_metrics_returns_all_fields() {
		let summary = Summary {
			total_files: 10,
			total_lines: 100,
			total_code_lines: 60,
			total_comment_lines: 20,
			total_blank_lines: 15,
			total_shebang_lines: 5,
			total_size: 4096,
			total_size_human: "4.0 KiB".to_string(),
			code_percentage: 60.0,
			comment_percentage: 20.0,
			blank_percentage: 15.0,
			shebang_percentage: 5.0,
		};
		let metrics: Vec<_> = summary.metrics().collect();
		assert_eq!(metrics.len(), 7);
		let labels: Vec<&str> = metrics.iter().map(|m| m.label).collect();
		assert_eq!(
			labels,
			vec![
				"Total Files",
				"Total Lines",
				"Code Lines",
				"Comment Lines",
				"Blank Lines",
				"Shebang Lines",
				"Total Size",
			]
		);
		// First two metrics (Total Files, Total Lines) have no percentage
		assert!(metrics[0].percentage.is_none());
		assert!(metrics[1].percentage.is_none());
		// Line-type metrics have percentages
		assert_eq!(metrics[2].percentage, Some(60.0));
		assert_eq!(metrics[3].percentage, Some(20.0));
		assert_eq!(metrics[4].percentage, Some(15.0));
		assert_eq!(metrics[5].percentage, Some(5.0));
		// Total Size has human_readable
		assert_eq!(metrics[6].human_readable, Some("4.0 KiB"));
		assert!(metrics[6].percentage.is_none());
	}

	#[test]
	fn summary_line_breakdown_pluralize() {
		let summary = Summary {
			total_files: 1,
			total_lines: 4,
			total_code_lines: 1,
			total_comment_lines: 1,
			total_blank_lines: 1,
			total_shebang_lines: 1,
			total_size: 0,
			total_size_human: "0 B".to_string(),
			code_percentage: 25.0,
			comment_percentage: 25.0,
			blank_percentage: 25.0,
			shebang_percentage: 25.0,
		};
		let ctx = FormatterContext::new(ViewOptions::default());
		let parts = summary.line_breakdown_parts(true, &ctx);
		// When count is 1 and pluralize=true, should use singular "line"
		assert_eq!(parts, vec!["1 code line", "1 comment line", "1 blank line", "1 shebang line"]);
	}

	#[test]
	fn summary_line_breakdown_no_pluralize() {
		let summary = Summary {
			total_files: 1,
			total_lines: 40,
			total_code_lines: 10,
			total_comment_lines: 15,
			total_blank_lines: 10,
			total_shebang_lines: 5,
			total_size: 0,
			total_size_human: "0 B".to_string(),
			code_percentage: 25.0,
			comment_percentage: 37.5,
			blank_percentage: 25.0,
			shebang_percentage: 12.5,
		};
		let ctx = FormatterContext::new(ViewOptions::default());
		let parts = summary.line_breakdown_parts(false, &ctx);
		// When pluralize=false, should use plural labels without "line"/"lines" suffix
		assert_eq!(parts, vec!["10 code", "15 comments", "10 blanks", "5 shebangs"]);
	}

	#[test]
	fn line_type_stats_labels() {
		let code = LineTypeStats { kind: LineType::Code, count: 10, percentage: 50.0 };
		assert_eq!(code.singular_label(), "code");
		assert_eq!(code.plural_label(), "code");
		assert_eq!(code.title_label(), "Code");

		let comment = LineTypeStats { kind: LineType::Comment, count: 5, percentage: 25.0 };
		assert_eq!(comment.singular_label(), "comment");
		assert_eq!(comment.plural_label(), "comments");
		assert_eq!(comment.title_label(), "Comments");

		let blank = LineTypeStats { kind: LineType::Blank, count: 3, percentage: 15.0 };
		assert_eq!(blank.singular_label(), "blank");
		assert_eq!(blank.plural_label(), "blanks");
		assert_eq!(blank.title_label(), "Blanks");

		let shebang = LineTypeStats { kind: LineType::Shebang, count: 2, percentage: 10.0 };
		assert_eq!(shebang.singular_label(), "shebang");
		assert_eq!(shebang.plural_label(), "shebangs");
		assert_eq!(shebang.title_label(), "Shebangs");
	}

	#[test]
	fn report_data_summary_verbosity_skips_languages() {
		let mut results = AnalysisResults::default();
		let lang = crate::langs::LANGUAGES.iter().find(|l| l.name == "Rust").unwrap();
		results.add_file(lang, &LineCounts { total: 10, code: 10, ..LineCounts::default() }, 100, None);

		let ctx = FormatterContext::new(ViewOptions::default());
		let report = ReportData::from_results(&results, Path::new("."), Verbosity::Summary, &ctx);

		assert_eq!(report.summary.total_files, 1);
		assert!(report.languages.is_empty());
	}

	#[test]
	fn report_data_regular_verbosity_includes_languages() {
		let mut results = AnalysisResults::default();
		let lang = crate::langs::LANGUAGES.iter().find(|l| l.name == "Rust").unwrap();
		results.add_file(lang, &LineCounts { total: 10, code: 10, ..LineCounts::default() }, 100, None);

		let ctx = FormatterContext::new(ViewOptions::default());
		let report = ReportData::from_results(&results, Path::new("."), Verbosity::Regular, &ctx);

		assert_eq!(report.summary.total_files, 1);
		assert_eq!(report.languages.len(), 1);
		assert_eq!(report.languages[0].name, "Rust");
		assert!(report.languages[0].files_detail.is_none());
	}
}
