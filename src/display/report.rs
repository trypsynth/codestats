use std::{
	collections::HashMap,
	path::{Component, Path},
};

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
		$($method_name:ident => $($field:ident).+ : $formatter:ident),* $(,)?
	}) => {
		impl $type {
			$(
				#[must_use]
				pub fn $method_name(&self, ctx: &FormatterContext) -> String {
					ctx.$formatter(self.$($field).+)
				}
			)*
		}
	};
}

#[derive(Debug)]
pub struct ReportData<'a> {
	pub analysis_path: String,
	pub summary: Summary,
	pub languages: Vec<LanguageRecord<'a>>,
	pub directories: Vec<DirRecord<'a>>,
	pub languages_hidden: Option<usize>,
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

#[derive(Debug)]
pub struct Summary {
	pub total_files: u64,
	pub counts: LineCounts,
	pub total_size: u64,
	pub total_size_human: String,
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
		Self {
			total_files: results.total_files(),
			counts: *results.counts(),
			total_size: results.total_size(),
			total_size_human: ctx.size(results.total_size()),
		}
	}

	fn iter_line_types(&self) -> impl Iterator<Item = LineTypeStats> {
		line_types(self.counts)
	}

	#[must_use]
	pub fn percentage_parts(&self, ctx: &FormatterContext) -> Vec<String> {
		self.iter_line_types()
			.map(|info| format!("{}% {}", ctx.percent(info.percentage), info.kind.plural_label()))
			.collect()
	}

	#[must_use]
	pub fn line_breakdown_parts(&self, pluralize: bool, ctx: &FormatterContext) -> Vec<String> {
		self.iter_line_types()
			.map(|info| {
				let formatted = ctx.number(info.count);
				if pluralize {
					format!("{formatted} {} {}", info.kind.singular_label(), pluralize_fn(info.count, "line", "lines"))
				} else {
					format!("{formatted} {}", info.kind.plural_label())
				}
			})
			.collect()
	}

	pub fn metrics(&self) -> impl Iterator<Item = SummaryMetric<'_>> {
		let line_metric = |label, kind| SummaryMetric {
			label,
			value: self.counts.get(kind),
			percentage: Some(self.counts.percentage(kind)),
			human_readable: None,
		};
		[
			SummaryMetric { label: "Total Files", value: self.total_files, percentage: None, human_readable: None },
			SummaryMetric { label: "Total Lines", value: self.counts.total, percentage: None, human_readable: None },
			line_metric("Code Lines", LineType::Code),
			line_metric("Comment Lines", LineType::Comment),
			line_metric("Blank Lines", LineType::Blank),
			line_metric("Shebang Lines", LineType::Shebang),
			SummaryMetric {
				label: "Total Size",
				value: self.total_size,
				percentage: None,
				human_readable: Some(self.total_size_human.as_str()),
			},
		]
		.into_iter()
	}
}

/// The value to order a row by for `key`. Rows without a file count (single files) sort by name for `Files`.
const fn sort_value<'a>(
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

#[derive(Debug)]
pub struct LanguageRecord<'a> {
	pub name: &'a str,
	pub files: u64,
	pub counts: LineCounts,
	pub size: u64,
	pub size_human: String,
	pub avg_lines_per_file: f64,
	pub code_percentage: f64,
	pub comment_percentage: f64,
	pub blank_percentage: f64,
	pub shebang_percentage: f64,
	pub files_detail: Option<Vec<FileRecord<'a>>>,
}

impl<'a> LanguageRecord<'a> {
	#[must_use]
	/// The languages to show, sorted and filtered by the view options, and how many were left out.
	pub(super) fn from_results(
		results: &'a AnalysisResults,
		verbosity: Verbosity,
		ctx: &FormatterContext,
	) -> (Vec<Self>, usize) {
		let mut stats_vec: Vec<_> = results.languages().map(|(lang, stats)| (lang.name, stats)).collect();
		let total = stats_vec.len();
		let sort_key = ctx.options.language_sort_key;
		apply_sort(
			&mut stats_vec,
			ctx.options.sort_direction,
			|(name, stats)| sort_value(sort_key, name, stats.counts(), Some(stats.files()), stats.size()),
			|a, b| a.0.cmp(b.0),
		);
		if let Some(min) = ctx.options.min_lines {
			stats_vec.retain(|(_, stats)| stats.counts().total >= min);
		}
		if let Some(n) = ctx.options.top_languages {
			stats_vec.truncate(n);
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
			counts: *counts,
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
		line_types(self.counts)
	}
}

impl_formatters!(LanguageRecord<'_> {
	format_files => files : number,
	format_lines => counts.total : number,
	format_code_lines => counts.code : number,
	format_comment_lines => counts.comment : number,
	format_blank_lines => counts.blank : number,
	format_shebang_lines => counts.shebang : number,
	format_size => size : number,
	format_code_percentage => code_percentage : percent,
	format_comment_percentage => comment_percentage : percent,
	format_blank_percentage => blank_percentage : percent,
	format_shebang_percentage => shebang_percentage : percent,
});

#[derive(Debug)]
pub struct FileRecord<'a> {
	pub path: &'a str,
	pub counts: LineCounts,
	pub size: u64,
	pub size_human: String,
}

impl<'a> FileRecord<'a> {
	fn from_stats(file: &'a FileStats, ctx: &FormatterContext) -> Self {
		Self { path: file.path(), counts: *file.counts(), size: file.size(), size_human: ctx.size(file.size()) }
	}
}

impl_formatters!(FileRecord<'_> {
	format_total_lines => counts.total : number,
	format_code_lines => counts.code : number,
	format_comment_lines => counts.comment : number,
	format_blank_lines => counts.blank : number,
	format_shebang_lines => counts.shebang : number,
	format_size => size : number,
});

impl_formatters!(DirRecord<'_> {
	format_files => files : number,
	format_lines => counts.total : number,
	format_code_lines => counts.code : number,
	format_comment_lines => counts.comment : number,
	format_blank_lines => counts.blank : number,
	format_shebang_lines => counts.shebang : number,
	format_size => size : number,
	format_code_percentage => code_percentage : percent,
	format_comment_percentage => comment_percentage : percent,
	format_blank_percentage => blank_percentage : percent,
	format_shebang_percentage => shebang_percentage : percent,
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

#[derive(Debug)]
pub struct DirRecord<'a> {
	pub path: String,
	pub files: u64,
	pub counts: LineCounts,
	pub size: u64,
	pub size_human: String,
	pub code_percentage: f64,
	pub comment_percentage: f64,
	pub blank_percentage: f64,
	pub shebang_percentage: f64,
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
					counts,
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
			|r| sort_value(sort_key, &r.path, &r.counts, Some(r.files), r.size),
			|a, b| a.path.cmp(&b.path),
		);
		if let Some(min) = ctx.options.min_lines {
			records.retain(|r| r.counts.total >= min);
		}
		if let Some(n) = ctx.options.top_languages {
			records.truncate(n);
		}
		let hidden = total.saturating_sub(records.len());
		(records, hidden)
	}

	pub fn line_types(&self) -> impl Iterator<Item = LineTypeStats> {
		line_types(self.counts)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::display::{formatting::FormatterContext, options::ViewOptions};

	fn summary(files: u64, [total, code, comment, blank, shebang]: [u64; 5], size: u64, size_human: &str) -> Summary {
		Summary {
			total_files: files,
			counts: LineCounts { total, code, comment, blank, shebang },
			total_size: size,
			total_size_human: size_human.to_owned(),
		}
	}

	#[test]
	fn summary_line_breakdown_parts_skip_zero() {
		let summary = summary(1, [10, 5, 2, 3, 0], 0, "0 B");
		let ctx = FormatterContext::new(ViewOptions::default());
		let parts = summary.line_breakdown_parts(true, &ctx);
		assert_eq!(parts, vec!["5 code lines", "2 comment lines", "3 blank lines"]);
	}

	#[test]
	fn summary_percentage_parts_skip_zero() {
		let summary = summary(1, [10, 5, 2, 3, 0], 0, "0 B");
		let ctx = FormatterContext::new(ViewOptions::default());
		let parts = summary.percentage_parts(&ctx);
		assert_eq!(parts, vec!["50.0% code", "20.0% comments", "30.0% blanks"]);
	}

	#[test]
	fn summary_metrics_returns_all_fields() {
		let summary = summary(10, [100, 60, 20, 15, 5], 4096, "4.0 KiB");
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
		let summary = summary(1, [4, 1, 1, 1, 1], 0, "0 B");
		let ctx = FormatterContext::new(ViewOptions::default());
		let parts = summary.line_breakdown_parts(true, &ctx);
		// When count is 1 and pluralize=true, should use singular "line"
		assert_eq!(parts, vec!["1 code line", "1 comment line", "1 blank line", "1 shebang line"]);
	}

	#[test]
	fn summary_line_breakdown_no_pluralize() {
		let summary = summary(1, [40, 10, 15, 10, 5], 0, "0 B");
		let ctx = FormatterContext::new(ViewOptions::default());
		let parts = summary.line_breakdown_parts(false, &ctx);
		// When pluralize=false, should use plural labels without "line"/"lines" suffix
		assert_eq!(parts, vec!["10 code", "15 comments", "10 blanks", "5 shebangs"]);
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
	fn min_lines_applies_before_top_languages() {
		let mut results = AnalysisResults::default();
		// Alphabetically, the two smallest languages come first.
		for (name, lines) in [("Ada", 1), ("C", 2), ("Go", 100), ("Python", 100), ("Rust", 100), ("Zig", 100)] {
			let lang = crate::langs::LANGUAGES.iter().find(|l| l.name == name).unwrap();
			results.add_file(lang, &LineCounts { total: lines, code: lines, ..LineCounts::default() }, 10, None);
		}
		let options = ViewOptions {
			language_sort_key: LanguageSortKey::Name,
			sort_direction: crate::display::SortDirection::Asc,
			min_lines: Some(50),
			top_languages: Some(3),
			..ViewOptions::default()
		};
		let ctx = FormatterContext::new(options);
		let report = ReportData::from_results(&results, Path::new("."), Verbosity::Regular, &ctx);
		let names: Vec<_> = report.languages.iter().map(|l| l.name).collect();
		assert_eq!(names, ["Go", "Python", "Rust"]);
		assert_eq!(report.languages_hidden, Some(3));
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
