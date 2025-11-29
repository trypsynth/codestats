use std::{io::Write, path::Path};

use anyhow::Result;

use super::{OutputFormatter, ReportData};
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
		writer: &mut dyn Write,
	) -> Result<()> {
		let report = ReportData::from_results(results, path, verbose);
		Self::write_overview(&report, writer)?;
		if report.languages.is_empty() {
			writeln!(writer, "No recognized programming languages found.")?;
			return Ok(());
		}
		Self::write_language_breakdown(&report, verbose, writer)?;
		Ok(())
	}
}

impl HumanFormatter {
	fn write_overview(report: &ReportData, writer: &mut dyn Write) -> Result<()> {
		let summary = &report.summary;
		let total_size_human = &summary.total_size_human;
		writeln!(
			writer,
			"Codestats for {}: {} {}, {} total {}, {} total size.",
			report.analysis_path,
			summary.total_files,
			utils::pluralize(summary.total_files, "file", "files"),
			summary.total_lines,
			utils::pluralize(summary.total_lines, "line", "lines"),
			total_size_human
		)?;
		let line_breakdown_parts = summary.line_breakdown_parts(true);
		if !line_breakdown_parts.is_empty() {
			writeln!(writer, "Line breakdown: {}.", line_breakdown_parts.join(", "))?;
		}
		let percentage_parts = summary.percentage_parts();
		if !percentage_parts.is_empty() {
			writeln!(writer, "Percentages: {}.", percentage_parts.join(", "))?;
		}
		Ok(())
	}

	fn write_language_breakdown(report: &ReportData, verbose: bool, writer: &mut dyn Write) -> Result<()> {
		writeln!(writer, "Language breakdown:")?;
		for language in &report.languages {
			Self::write_language_stats(language, &report.summary, verbose, writer)?;
		}
		Ok(())
	}

	fn write_language_stats(
		language: &LanguageRecord,
		summary: &Summary,
		verbose: bool,
		writer: &mut dyn Write,
	) -> Result<()> {
		let file_pct = utils::percentage(language.files, summary.total_files);
		let line_pct = utils::percentage(language.lines, summary.total_lines);
		let size_pct = utils::percentage(language.size, summary.total_size);
		let size_human = &language.size_human;
		writeln!(writer, "{}:", language.name)?;
		writeln!(
			writer,
			"\tFiles: {} {} ({file_pct:.1}% of total).",
			language.files,
			utils::pluralize(language.files, "file", "files")
		)?;
		writeln!(
			writer,
			"\tLines: {} {} ({line_pct:.1}% of total).",
			language.lines,
			utils::pluralize(language.lines, "line", "lines")
		)?;
		writeln!(writer, "\tSize: {size_human} ({size_pct:.1}% of total).")?;
		writeln!(writer, "\tLine breakdown:")?;
		if language.code_lines > 0 {
			writeln!(writer, "\t\tCode: {} lines ({:.1}%).", language.code_lines, language.code_percentage)?;
		}
		if language.comment_lines > 0 {
			writeln!(writer, "\t\tComments: {} lines ({:.1}%).", language.comment_lines, language.comment_percentage)?;
		}
		if language.blank_lines > 0 {
			writeln!(writer, "\t\tBlanks: {} lines ({:.1}%).", language.blank_lines, language.blank_percentage)?;
		}
		if language.shebang_lines > 0 {
			writeln!(writer, "\t\tShebangs: {} lines ({:.1}%).", language.shebang_lines, language.shebang_percentage)?;
		}
		if verbose {
			Self::write_file_breakdown(language, summary, writer)?;
		}
		Ok(())
	}

	fn write_file_breakdown(language: &LanguageRecord, summary: &Summary, writer: &mut dyn Write) -> Result<()> {
		writeln!(writer, "\tFile breakdown:")?;
		let Some(files) = &language.files_detail else {
			return Ok(());
		};
		for file_stat in files {
			let file_pct = utils::percentage(file_stat.total_lines, summary.total_lines);
			let size_human = &file_stat.size_human;
			writeln!(
				writer,
				"\t\t{}: {} lines, {} ({:.1}% of total lines).",
				file_stat.path, file_stat.total_lines, size_human, file_pct
			)?;
		}
		Ok(())
	}
}
