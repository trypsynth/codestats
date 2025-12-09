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

struct LanguageLineTypeInfo {
	label: &'static str,
	count: u64,
	percentage: f64,
}

impl HumanFormatter {
	/// Formats a list into a human-friendly phrase.
	/// - 0 items => empty string.
	/// - 1 item => "A"
	/// - 2 items => "A and B"
	/// - 3+ => "A, B, and C"
	fn format_list(items: &[String]) -> String {
		match items {
			[] => String::new(),
			[first] => first.clone(),
			[first, second] => format!("{first} and {second}"),
			_ => {
				let mut result = items[..items.len() - 1].join(", ");
				result.push_str(", and ");
				result.push_str(&items[items.len() - 1]);
				result
			}
		}
	}

	/// Iterator over language line types for consistent formatting.
	fn iter_language_line_types(lang: &LanguageRecord<'_>) -> impl Iterator<Item = LanguageLineTypeInfo> {
		[
			LanguageLineTypeInfo { label: "Code", count: lang.code_lines, percentage: lang.code_percentage },
			LanguageLineTypeInfo { label: "Comments", count: lang.comment_lines, percentage: lang.comment_percentage },
			LanguageLineTypeInfo { label: "Blanks", count: lang.blank_lines, percentage: lang.blank_percentage },
			LanguageLineTypeInfo { label: "Shebangs", count: lang.shebang_lines, percentage: lang.shebang_percentage },
		]
		.into_iter()
		.filter(|info| info.count > 0)
	}

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
			let breakdown = Self::format_list(&line_breakdown_parts);
			writeln!(writer, "Line breakdown: {breakdown}.")?;
		}
		let percentage_parts = summary.percentage_parts(ctx);
		if !percentage_parts.is_empty() {
			let percentages = Self::format_list(&percentage_parts);
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
		for line_type in Self::iter_language_line_types(language) {
			writeln!(
				writer,
				"\t\t{}: {} lines ({}%).",
				line_type.label,
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

#[cfg(test)]
mod tests {
	use super::HumanFormatter;

	#[test]
	fn test_format_list() {
		assert!(HumanFormatter::format_list(&[]).is_empty());
		let mut input = vec!["one".to_string()];
		assert_eq!(HumanFormatter::format_list(&input), "one");
		input = vec!["one".to_string(), "two".to_string()];
		assert_eq!(HumanFormatter::format_list(&input), "one and two");
		input = vec!["one".to_string(), "two".to_string(), "three".to_string()];
		assert_eq!(HumanFormatter::format_list(&input), "one, two, and three");
	}
}
