use std::{borrow::Cow, io::Write, path::Path};

use anyhow::Result;

use super::{FormatterContext, OutputFormatter, ReportData, ViewOptions};
use crate::{
	analysis::{AnalysisResults, stats::percentage},
	display::{
		formatting::pluralize,
		report::{LanguageRecord, Summary},
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
		verbose: bool,
		view_options: ViewOptions,
		writer: &mut dyn Write,
	) -> Result<()> {
		let (ctx, report) = self.prepare_report(results, path, verbose, view_options);
		Self::write_overview(&report, &ctx, writer)?;
		if report.languages.is_empty() {
			writeln!(writer, "No recognized programming languages found.")?;
			return Ok(());
		}
		Self::write_language_breakdown(&report, &ctx, verbose, writer)?;
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
		if let Some(unrecognized) = summary.unrecognized_files {
			writeln!(writer, "Unrecognized files: {}.", ctx.number(unrecognized))?;
		}
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
		Ok(())
	}

	fn write_language_stats(
		language: &LanguageRecord,
		summary: &Summary,
		ctx: &FormatterContext,
		verbose: bool,
		writer: &mut dyn Write,
	) -> Result<()> {
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
			"\tFiles: {} {} ({}% of total).",
			ctx.number(language.files),
			pluralize(language.files, "file", "files"),
			file_pct_str
		)?;
		writeln!(
			writer,
			"\tLines: {} {} ({}% of total).",
			ctx.number(language.lines),
			pluralize(language.lines, "line", "lines"),
			line_pct_str
		)?;
		writeln!(writer, "\tAverage lines per file: {:.1}.", language.avg_lines_per_file)?;
		writeln!(writer, "\tSize: {size_human} ({size_pct_str}% of total).")?;
		writeln!(writer, "\tLine breakdown:")?;
		for line_type in language.line_types() {
			writeln!(
				writer,
				"\t\t{}: {} lines ({}%).",
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

	fn write_file_breakdown(
		language: &LanguageRecord,
		summary: &Summary,
		ctx: &FormatterContext,
		writer: &mut dyn Write,
	) -> Result<()> {
		writeln!(writer, "\tFile breakdown:")?;
		let Some(files) = &language.files_detail else {
			return Ok(());
		};
		for file_stat in files {
			let file_pct = percentage(file_stat.total_lines, summary.total_lines);
			let file_pct_str = ctx.percent(file_pct);
			let size_human = &file_stat.size_human;
			writeln!(
				writer,
				"\t\t{}: {} lines, {} ({}% of total lines).",
				file_stat.path,
				ctx.number(file_stat.total_lines),
				size_human,
				file_pct_str
			)?;
		}
		Ok(())
	}
}
