use std::{borrow::Cow, io::Write, path::Path};

use anyhow::Result;

use super::{FormatterContext, OutputFormatter, ReportData, Verbosity, ViewOptions};
use crate::{
	analysis::{AnalysisResults, stats::percentage},
	display::{
		formatting::pluralize,
		report::{DirRecord, LanguageRecord, Summary},
	},
};

pub struct HumanFormatter;

fn join_with_commas_and(parts: &[String]) -> Option<Cow<'_, str>> {
	match parts {
		[] => None,
		[first] => Some(Cow::Borrowed(first.as_str())),
		[first, second] => Some(Cow::Owned(format!("{first} and {second}"))),
		items => {
			let mut result = items[..items.len() - 1].join(", ");
			result.push_str(", and ");
			result.push_str(&items[items.len() - 1]);
			Some(Cow::Owned(result))
		}
	}
}

impl OutputFormatter for HumanFormatter {
	fn write_output(
		&self,
		results: &AnalysisResults,
		path: &Path,
		view_options: ViewOptions,
		writer: &mut dyn Write,
	) -> Result<()> {
		let (ctx, report) = self.prepare_report(results, path, view_options);
		Self::write_overview(&report, &ctx, writer)?;
		if view_options.verbosity == Verbosity::Summary {
			return Ok(());
		}
		if view_options.by_dir {
			if report.directories.is_empty() {
				writeln!(writer, "No recognized programming languages found.")?;
			} else {
				Self::write_dir_breakdown(&report, &ctx, writer)?;
			}
			return Ok(());
		}
		if report.languages.is_empty() {
			writeln!(writer, "No recognized programming languages found.")?;
			return Ok(());
		}
		Self::write_language_breakdown(&report, &ctx, view_options.verbosity == Verbosity::Verbose, writer)?;
		Ok(())
	}
}

impl HumanFormatter {
	fn write_overview(report: &ReportData, ctx: &FormatterContext, writer: &mut dyn Write) -> Result<()> {
		let summary = &report.summary;
		let total_size_human = &summary.total_size_human;
		writeln!(
			writer,
			"Codestats for {}: {} {}, {} total {}, {} total size.",
			report.analysis_path,
			ctx.number(summary.total_files),
			pluralize(summary.total_files, "file", "files"),
			ctx.number(summary.total_lines),
			pluralize(summary.total_lines, "line", "lines"),
			total_size_human
		)?;
		let line_breakdown_parts = summary.line_breakdown_parts(true, ctx);
		if let Some(breakdown) = join_with_commas_and(&line_breakdown_parts) {
			writeln!(writer, "Line breakdown: {breakdown}.")?;
		}
		let percentage_parts = summary.percentage_parts(ctx);
		if let Some(percentages) = join_with_commas_and(&percentage_parts) {
			writeln!(writer, "Percentages: {percentages}.")?;
		}
		Ok(())
	}

	fn write_language_breakdown(
		report: &ReportData,
		ctx: &FormatterContext,
		verbose: bool,
		writer: &mut dyn Write,
	) -> Result<()> {
		writeln!(writer, "Language breakdown:")?;
		for language in &report.languages {
			Self::write_language_stats(language, &report.summary, ctx, verbose, writer)?;
		}
		if let Some(n) = report.languages_hidden {
			writeln!(writer, "({n} {} not shown)", pluralize(n as u64, "language", "languages"))?;
		}
		Ok(())
	}

	fn write_language_stats(
		language: &LanguageRecord,
		summary: &Summary,
		ctx: &FormatterContext,
		verbose: bool,
		writer: &mut dyn Write,
	) -> Result<()> {
		let i1 = ctx.indent(1);
		let i2 = ctx.indent(2);
		let file_pct = percentage(language.files, summary.total_files);
		let line_pct = percentage(language.lines, summary.total_lines);
		let size_pct = percentage(language.size, summary.total_size);
		let size_human = &language.size_human;
		let file_pct_str = ctx.percent(file_pct);
		let line_pct_str = ctx.percent(line_pct);
		let size_pct_str = ctx.percent(size_pct);
		writeln!(writer, "{}:", language.name)?;
		writeln!(
			writer,
			"{i1}Files: {} {} ({}% of total).",
			ctx.number(language.files),
			pluralize(language.files, "file", "files"),
			file_pct_str
		)?;
		writeln!(
			writer,
			"{i1}Lines: {} {} ({}% of total).",
			ctx.number(language.lines),
			pluralize(language.lines, "line", "lines"),
			line_pct_str
		)?;
		writeln!(writer, "{i1}Average lines per file: {:.1}.", language.avg_lines_per_file)?;
		writeln!(writer, "{i1}Size: {size_human} ({size_pct_str}% of total).")?;
		writeln!(writer, "{i1}Line breakdown:")?;
		for line_type in language.line_types() {
			writeln!(
				writer,
				"{i2}{}: {} lines ({}%).",
				line_type.title_label(),
				ctx.number(line_type.count),
				ctx.percent(line_type.percentage)
			)?;
		}
		if verbose {
			Self::write_file_breakdown(language, summary, ctx, writer)?;
		}
		Ok(())
	}

	fn write_dir_breakdown(report: &ReportData, ctx: &FormatterContext, writer: &mut dyn Write) -> Result<()> {
		writeln!(writer, "Directory breakdown:")?;
		for dir in &report.directories {
			Self::write_dir_stats(dir, &report.summary, ctx, writer)?;
		}
		if let Some(n) = report.dirs_hidden {
			writeln!(writer, "({n} {} not shown)", pluralize(n as u64, "directory", "directories"))?;
		}
		Ok(())
	}

	fn write_dir_stats(
		dir: &DirRecord,
		summary: &Summary,
		ctx: &FormatterContext,
		writer: &mut dyn Write,
	) -> Result<()> {
		let i1 = ctx.indent(1);
		let i2 = ctx.indent(2);
		let file_pct = percentage(dir.files, summary.total_files);
		let line_pct = percentage(dir.lines, summary.total_lines);
		let size_pct = percentage(dir.size, summary.total_size);
		let size_human = &dir.size_human;
		writeln!(writer, "{}:", dir.path)?;
		writeln!(
			writer,
			"{i1}Files: {} {} ({}% of total).",
			ctx.number(dir.files),
			pluralize(dir.files, "file", "files"),
			ctx.percent(file_pct)
		)?;
		writeln!(
			writer,
			"{i1}Lines: {} {} ({}% of total).",
			ctx.number(dir.lines),
			pluralize(dir.lines, "line", "lines"),
			ctx.percent(line_pct)
		)?;
		writeln!(writer, "{i1}Size: {size_human} ({}% of total).", ctx.percent(size_pct))?;
		writeln!(writer, "{i1}Line breakdown:")?;
		for line_type in dir.line_types() {
			writeln!(
				writer,
				"{i2}{}: {} lines ({}%).",
				line_type.title_label(),
				ctx.number(line_type.count),
				ctx.percent(line_type.percentage)
			)?;
		}
		Ok(())
	}

	fn write_file_breakdown(
		language: &LanguageRecord,
		summary: &Summary,
		ctx: &FormatterContext,
		writer: &mut dyn Write,
	) -> Result<()> {
		let i1 = ctx.indent(1);
		let i2 = ctx.indent(2);
		writeln!(writer, "{i1}File breakdown:")?;
		let Some(files) = &language.files_detail else {
			return Ok(());
		};
		for file_stat in files {
			let file_pct = percentage(file_stat.total_lines, summary.total_lines);
			let file_pct_str = ctx.percent(file_pct);
			let size_human = &file_stat.size_human;
			writeln!(
				writer,
				"{i2}{}: {} lines, {} ({}% of total lines).",
				file_stat.path,
				ctx.number(file_stat.total_lines),
				size_human,
				file_pct_str
			)?;
		}
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use std::path::Path;

	use super::{HumanFormatter, join_with_commas_and};
	use crate::{
		analysis::{AnalysisResults, stats::FileContribution},
		display::{OutputFormatter, ViewOptions, options::IndentStyle},
	};

	#[test]
	fn join_with_commas_and_formats_lists() {
		let empty: Vec<String> = Vec::new();
		assert!(join_with_commas_and(&empty).is_none());
		let single = vec!["alpha".to_string()];
		assert_eq!(join_with_commas_and(&single).as_deref(), Some("alpha"));
		let pair = vec!["alpha".to_string(), "beta".to_string()];
		assert_eq!(join_with_commas_and(&pair).as_deref(), Some("alpha and beta"));
		let triple = vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()];
		assert_eq!(join_with_commas_and(&triple).as_deref(), Some("alpha, beta, and gamma"));
	}

	#[test]
	fn human_output_uses_configured_indent() {
		let mut results = AnalysisResults::default();
		let lang = crate::langs::LANGUAGES.iter().find(|l| l.name == "Rust").unwrap();
		let contribution = FileContribution::new(12, 10, 0, 2, 0, 100);
		results.add_file_stats(lang, contribution, None);
		let mut options = ViewOptions::default();
		options.indent_style = IndentStyle::Spaces(2);
		let formatter = HumanFormatter;
		let mut buf = Vec::new();
		formatter.write_output(&results, Path::new("."), options, &mut buf).unwrap();
		let output = String::from_utf8(buf).unwrap();
		assert!(output.contains("  Files:"), "expected 2-space indent for Files, got:\n{output}");
		assert!(!output.contains("\tFiles:"), "should not contain tab-indented Files");
	}
}
