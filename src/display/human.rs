use std::{io::Write, path::Path};

use anyhow::Result;

use super::{FormatterContext, OutputFormatter, ReportData, ViewOptions};
use crate::{
	analysis::AnalysisResults,
	display::report::{LanguageRecord, Summary},
	utils,
};

pub struct HumanFormatter;

impl OutputFormatter for HumanFormatter {
	fn write_output(
		&self,
		results: &AnalysisResults,
		path: &Path,
		verbose: bool,
		view_options: ViewOptions,
		writer: &mut dyn Write,
	) -> Result<()> {
		let ctx = FormatterContext::new(view_options);
		let report = ReportData::from_results(results, path, verbose, &ctx);
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
			utils::pluralize(summary.total_files, "file", "files"),
			ctx.number(summary.total_lines),
			utils::pluralize(summary.total_lines, "line", "lines"),
			total_size_human
		)?;
		let line_breakdown_parts = summary.line_breakdown_parts(true, ctx);
		if !line_breakdown_parts.is_empty() {
			writeln!(writer, "Line breakdown: {}.", line_breakdown_parts.join(", "))?;
		}
		let percentage_parts = summary.percentage_parts(ctx);
		if !percentage_parts.is_empty() {
			writeln!(writer, "Percentages: {}.", percentage_parts.join(", "))?;
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
		let file_pct = utils::percentage(language.files, summary.total_files);
		let line_pct = utils::percentage(language.lines, summary.total_lines);
		let size_pct = utils::percentage(language.size, summary.total_size);
		let size_human = &language.size_human;
		let file_pct_str = ctx.percent(file_pct);
		let line_pct_str = ctx.percent(line_pct);
		let size_pct_str = ctx.percent(size_pct);
		writeln!(writer, "{}:", language.name)?;
		writeln!(
			writer,
			"\tFiles: {} {} ({}% of total).",
			ctx.number(language.files),
			utils::pluralize(language.files, "file", "files"),
			file_pct_str
		)?;
		writeln!(
			writer,
			"\tLines: {} {} ({}% of total).",
			ctx.number(language.lines),
			utils::pluralize(language.lines, "line", "lines"),
			line_pct_str
		)?;
		writeln!(writer, "\tSize: {size_human} ({size_pct_str}% of total).")?;
		writeln!(writer, "\tLine breakdown:")?;
		if language.code_lines > 0 {
			writeln!(
				writer,
				"\t\tCode: {} lines ({}%).",
				ctx.number(language.code_lines),
				ctx.percent(language.code_percentage)
			)?;
		}
		if language.comment_lines > 0 {
			writeln!(
				writer,
				"\t\tComments: {} lines ({}%).",
				ctx.number(language.comment_lines),
				ctx.percent(language.comment_percentage)
			)?;
		}
		if language.blank_lines > 0 {
			writeln!(
				writer,
				"\t\tBlanks: {} lines ({}%).",
				ctx.number(language.blank_lines),
				ctx.percent(language.blank_percentage)
			)?;
		}
		if language.shebang_lines > 0 {
			writeln!(
				writer,
				"\t\tShebangs: {} lines ({}%).",
				ctx.number(language.shebang_lines),
				ctx.percent(language.shebang_percentage)
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
			let file_pct = utils::percentage(file_stat.total_lines, summary.total_lines);
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
