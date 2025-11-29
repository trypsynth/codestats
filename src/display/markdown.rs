use std::{io::Write, path::Path};

use anyhow::Result;

use super::{OutputFormatter, ReportData};
use crate::{
	analysis::AnalysisResults,
	display::report::{LanguageRecord, Summary},
};

pub struct MarkdownFormatter;

impl OutputFormatter for MarkdownFormatter {
	fn write_output(
		&self,
		results: &AnalysisResults,
		path: &Path,
		verbose: bool,
		writer: &mut dyn Write,
	) -> Result<()> {
		let report = ReportData::from_results(results, path, verbose);
		Self::write_markdown(&report, verbose, writer)
	}
}

impl MarkdownFormatter {
	fn write_markdown(report: &ReportData, verbose: bool, writer: &mut dyn Write) -> Result<()> {
		writeln!(writer, "# Codestats for `{}`", report.analysis_path)?;
		Self::write_summary(&report.summary, writer)?;
		if report.languages.is_empty() {
			writeln!(writer, "\n_No recognized programming languages found._")?;
			return Ok(());
		}
		Self::write_language_table(&report.languages, writer)?;
		if verbose {
			Self::write_file_tables(&report.languages, writer)?;
		}
		Ok(())
	}

	fn write_summary(summary: &Summary, writer: &mut dyn Write) -> Result<()> {
		writeln!(writer, "\n## Summary")?;
		writeln!(writer, "- Files: {}", summary.total_files,)?;
		writeln!(writer, "- Lines: {}", summary.total_lines,)?;
		writeln!(writer, "- Size: {}", summary.total_size_human)?;
		let line_breakdown = summary.line_breakdown_parts(false);
		if !line_breakdown.is_empty() {
			writeln!(writer, "- Line types: {}", line_breakdown.join(", "))?;
		}
		let percentage_parts = summary.percentage_parts();
		if !percentage_parts.is_empty() {
			writeln!(writer, "- Totals: {}", percentage_parts.join(", "))?;
		}
		Ok(())
	}

	fn write_language_table(languages: &[LanguageRecord], writer: &mut dyn Write) -> Result<()> {
		writeln!(writer, "\n## Languages")?;
		writeln!(writer, "| Language | Files | Lines | Code % | Comment % | Blank % | Shebang % | Size |")?;
		writeln!(writer, "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |")?;
		for lang in languages {
			writeln!(
				writer,
				"| {} | {} | {} | {:.1}% | {:.1}% | {:.1}% | {:.1}% | {} |",
				escape_cell(lang.name),
				lang.files,
				lang.lines,
				lang.code_percentage,
				lang.comment_percentage,
				lang.blank_percentage,
				lang.shebang_percentage,
				escape_cell(&lang.size_human)
			)?;
		}
		Ok(())
	}

	fn write_file_tables(languages: &[LanguageRecord], writer: &mut dyn Write) -> Result<()> {
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
					escape_cell(&file.size_human)
				)?;
			}
		}
		Ok(())
	}
}

fn escape_cell(value: &str) -> String {
	value.replace('|', "\\|")
}
