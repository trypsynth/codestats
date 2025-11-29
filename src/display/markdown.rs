use std::{io::Write, path::Path};

use anyhow::Result;

use super::{FormatterContext, OutputFormatter, ReportData, ViewOptions};
use crate::{
	analysis::AnalysisResults,
	display::report::{FormattedLanguage, Summary},
};

pub struct MarkdownFormatter;

impl OutputFormatter for MarkdownFormatter {
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
		let languages = report.formatted_languages(&ctx);
		Self::write_markdown(&report.summary, &report.analysis_path, &languages, verbose, &ctx, writer)
	}
}

impl MarkdownFormatter {
	fn write_markdown(
		summary: &Summary,
		analysis_path: &str,
		languages: &[FormattedLanguage],
		verbose: bool,
		ctx: &FormatterContext,
		writer: &mut dyn Write,
	) -> Result<()> {
		writeln!(writer, "# Codestats for `{analysis_path}`")?;
		Self::write_summary(summary, ctx, writer)?;
		if languages.is_empty() {
			writeln!(writer, "\n_No recognized programming languages found._")?;
			return Ok(());
		}
		Self::write_language_table(languages, writer)?;
		if verbose {
			Self::write_file_tables(languages, writer)?;
		}
		Ok(())
	}

	fn write_summary(summary: &Summary, ctx: &FormatterContext, writer: &mut dyn Write) -> Result<()> {
		writeln!(writer, "\n## Summary")?;
		writeln!(writer, "- Files: {}", ctx.number(summary.total_files),)?;
		writeln!(writer, "- Lines: {}", ctx.number(summary.total_lines),)?;
		writeln!(writer, "- Size: {}", summary.total_size_human)?;
		let line_breakdown = summary.line_breakdown_parts(false, ctx);
		if !line_breakdown.is_empty() {
			writeln!(writer, "- Line types: {}", line_breakdown.join(", "))?;
		}
		let percentage_parts = summary.percentage_parts(ctx);
		if !percentage_parts.is_empty() {
			writeln!(writer, "- Totals: {}", percentage_parts.join(", "))?;
		}
		Ok(())
	}

	fn write_language_table(languages: &[FormattedLanguage], writer: &mut dyn Write) -> Result<()> {
		writeln!(writer, "\n## Languages")?;
		writeln!(writer, "| Language | Files | Lines | Code % | Comment % | Blank % | Shebang % | Size |")?;
		writeln!(writer, "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |")?;
		for lang in languages {
			writeln!(
				writer,
				"| {} | {} | {} | {}% | {}% | {}% | {}% | {} |",
				escape_cell(lang.name),
				lang.files,
				lang.lines,
				lang.code_percentage,
				lang.comment_percentage,
				lang.blank_percentage,
				lang.shebang_percentage,
				escape_cell(lang.size_human)
			)?;
		}
		Ok(())
	}

	fn write_file_tables(languages: &[FormattedLanguage], writer: &mut dyn Write) -> Result<()> {
		writeln!(writer, "\n## Files")?;
		for lang in languages {
			let Some(files) = &lang.files_detail else {
				continue;
			};
			writeln!(writer, "\n#### {}", lang.name)?;
			writeln!(
				writer,
				"| File | Total lines | Code lines | Comment lines | Blank lines | Shebang lines | Size |"
			)?;
			writeln!(writer, "| --- | ---: | ---: | ---: | ---: | ---: | ---: |")?;
			for file in files {
				writeln!(
					writer,
					"| {} | {} | {} | {} | {} | {} | {} |",
					escape_cell(file.path),
					file.total_lines,
					file.code_lines,
					file.comment_lines,
					file.blank_lines,
					file.shebang_lines,
					escape_cell(file.size_human)
				)?;
			}
		}
		Ok(())
	}
}

fn escape_cell(value: &str) -> String {
	value.replace('|', "\\|")
}
