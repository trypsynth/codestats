use std::{cmp::Reverse, io::Write, path::Path};

use super::OutputFormatter;
use crate::{
	analysis::{AnalysisResults, LanguageStats},
	utils,
};

/// Human-readable formatter
pub struct HumanFormatter;

impl OutputFormatter for HumanFormatter {
	fn write_output(
		&self,
		results: &AnalysisResults,
		path: &Path,
		verbose: bool,
		writer: &mut dyn Write,
	) -> anyhow::Result<()> {
		Self::write_overview(results, path, writer)?;
		if results.language_stats().is_empty() {
			writeln!(writer, "No recognized programming languages found.")?;
			return Ok(());
		}
		Self::write_language_breakdown(results, verbose, writer)?;
		Ok(())
	}
}

impl HumanFormatter {
	fn write_overview(results: &AnalysisResults, path: &Path, writer: &mut dyn Write) -> anyhow::Result<()> {
		let total_size_human = results.total_size_human();
		writeln!(
			writer,
			"Codestats for {}: {} {}, {} total {}, {} total size.",
			path.display(),
			results.total_files(),
			utils::pluralize(results.total_files(), "file", "files"),
			results.total_lines(),
			utils::pluralize(results.total_lines(), "line", "lines"),
			total_size_human
		)?;
		let line_breakdown_parts = Self::build_line_breakdown_parts(results);
		if !line_breakdown_parts.is_empty() {
			writeln!(writer, "Line breakdown: {}.", line_breakdown_parts.join(", "))?;
		}
		let percentage_parts = Self::build_percentage_parts(results);
		if !percentage_parts.is_empty() {
			writeln!(writer, "Percentages: {}.", percentage_parts.join(", "))?;
		}
		Ok(())
	}

	fn write_language_breakdown(
		results: &AnalysisResults,
		verbose: bool,
		writer: &mut dyn Write,
	) -> anyhow::Result<()> {
		writeln!(writer, "Language breakdown:")?;
		for (lang, lang_stats) in results.languages_by_lines() {
			Self::write_language_stats(lang, lang_stats, results, verbose, writer)?;
		}
		Ok(())
	}

	fn write_language_stats(
		lang: &str,
		lang_stats: &LanguageStats,
		overall_results: &AnalysisResults,
		verbose: bool,
		writer: &mut dyn Write,
	) -> anyhow::Result<()> {
		let file_pct = utils::percentage(lang_stats.files(), overall_results.total_files());
		let line_pct = utils::percentage(lang_stats.lines(), overall_results.total_lines());
		let size_pct = utils::percentage(lang_stats.size(), overall_results.total_size());
		let size_human = lang_stats.size_human();
		writeln!(writer, "{lang}:")?;
		writeln!(
			writer,
			"\tFiles: {} {} ({file_pct:.1}% of total).",
			lang_stats.files(),
			utils::pluralize(lang_stats.files(), "file", "files")
		)?;
		writeln!(
			writer,
			"\tLines: {} {} ({line_pct:.1}% of total).",
			lang_stats.lines(),
			utils::pluralize(lang_stats.lines(), "line", "lines")
		)?;
		writeln!(writer, "\tSize: {size_human} ({size_pct:.1}% of total).")?;
		writeln!(writer, "\tLine breakdown:")?;
		if lang_stats.code_lines() > 0 {
			writeln!(writer, "\t\tCode: {} lines ({:.1}%).", lang_stats.code_lines(), lang_stats.code_percentage())?;
		}
		if lang_stats.comment_lines() > 0 {
			writeln!(
				writer,
				"\t\tComments: {} lines ({:.1}%).",
				lang_stats.comment_lines(),
				lang_stats.comment_percentage()
			)?;
		}
		if lang_stats.blank_lines() > 0 {
			writeln!(
				writer,
				"\t\tBlanks: {} lines ({:.1}%).",
				lang_stats.blank_lines(),
				lang_stats.blank_percentage()
			)?;
		}
		if lang_stats.shebang_lines() > 0 {
			writeln!(
				writer,
				"\t\tShebangs: {} lines ({:.1}%).",
				lang_stats.shebang_lines(),
				lang_stats.shebang_percentage()
			)?;
		}
		if verbose {
			Self::write_file_breakdown(lang_stats, overall_results, writer)?;
		}
		Ok(())
	}

	fn write_file_breakdown(
		lang_stats: &LanguageStats,
		overall_results: &AnalysisResults,
		writer: &mut dyn Write,
	) -> anyhow::Result<()> {
		writeln!(writer, "\tFile breakdown:")?;
		let mut files: Vec<_> = lang_stats.files_list().iter().collect();
		files.sort_by_key(|b| Reverse(b.total_lines()));
		for file_stat in files {
			let file_pct = utils::percentage(file_stat.total_lines(), overall_results.total_lines());
			let size_human = file_stat.size_human();
			writeln!(
				writer,
				"\t\t{}: {} lines, {} ({:.1}% of total lines).",
				file_stat.path(),
				file_stat.total_lines(),
				size_human,
				file_pct
			)?;
		}
		Ok(())
	}

	fn build_line_breakdown_parts(results: &AnalysisResults) -> Vec<String> {
		let mut parts = Vec::with_capacity(4);
		if results.total_code_lines() > 0 {
			parts.push(format!(
				"{} code {}",
				results.total_code_lines(),
				utils::pluralize(results.total_code_lines(), "line", "lines")
			));
		}
		if results.total_comment_lines() > 0 {
			parts.push(format!(
				"{} comment {}",
				results.total_comment_lines(),
				utils::pluralize(results.total_comment_lines(), "line", "lines")
			));
		}
		if results.total_blank_lines() > 0 {
			parts.push(format!(
				"{} blank {}",
				results.total_blank_lines(),
				utils::pluralize(results.total_blank_lines(), "line", "lines")
			));
		}
		if results.total_shebang_lines() > 0 {
			parts.push(format!(
				"{} shebang {}",
				results.total_shebang_lines(),
				utils::pluralize(results.total_shebang_lines(), "line", "lines")
			));
		}
		parts
	}

	fn build_percentage_parts(results: &AnalysisResults) -> Vec<String> {
		let mut parts = Vec::with_capacity(4);
		if results.total_code_lines() > 0 {
			parts.push(format!("{:.1}% code", results.code_percentage()));
		}
		if results.total_comment_lines() > 0 {
			parts.push(format!("{:.1}% comments", results.comment_percentage()));
		}
		if results.total_blank_lines() > 0 {
			parts.push(format!("{:.1}% blanks", results.blank_percentage()));
		}
		if results.total_shebang_lines() > 0 {
			parts.push(format!("{:.1}% shebangs", results.shebang_percentage()));
		}
		parts
	}
}
