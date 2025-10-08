use std::{cmp::Reverse, fmt::Write, path::Path};

use super::OutputFormatter;
use crate::{
	analysis::{AnalysisResults, LanguageStats},
	utils,
};

/// Human-readable formatter
pub struct HumanFormatter;

impl OutputFormatter for HumanFormatter {
	fn format(&self, results: &AnalysisResults, path: &Path, verbose: bool) -> anyhow::Result<String> {
		let mut output = String::new();
		output.push_str(&Self::format_overview(results, path));
		if results.language_stats().is_empty() {
			output.push_str("No recognized programming languages found.\n");
			return Ok(output);
		}
		output.push_str(&Self::format_language_breakdown(results, verbose));
		Ok(output)
	}
}

impl HumanFormatter {
	fn format_overview(results: &AnalysisResults, path: &Path) -> String {
		let mut output = format!(
			"Codestats for {}: {} {}, {} total {}, {} total size.\n",
			path.display(),
			results.total_files(),
			utils::pluralize(results.total_files(), "file", "files"),
			results.total_lines(),
			utils::pluralize(results.total_lines(), "line", "lines"),
			results.total_size_human()
		);
		let line_breakdown_parts = Self::build_line_breakdown_parts(results);
		if !line_breakdown_parts.is_empty() {
			writeln!(output, "Line breakdown: {}", line_breakdown_parts.join(", ")).ok();
		}
		let percentage_parts = Self::build_percentage_parts(results);
		if !percentage_parts.is_empty() {
			writeln!(output, "Percentages: {}", percentage_parts.join(", ")).ok();
		}
		output
	}

	fn format_language_breakdown(results: &AnalysisResults, verbose: bool) -> String {
		let mut output = String::from("Language breakdown:\n");
		for (lang, lang_stats) in results.languages_by_lines() {
			output.push_str(&Self::format_language_stats(lang, lang_stats, results, verbose));
		}
		output
	}

	fn format_language_stats(
		lang: &str,
		lang_stats: &LanguageStats,
		overall_results: &AnalysisResults,
		verbose: bool,
	) -> String {
		let mut output = String::new();
		let file_pct = utils::percentage(lang_stats.files(), overall_results.total_files());
		let line_pct = utils::percentage(lang_stats.lines(), overall_results.total_lines());
		let size_pct = utils::percentage(lang_stats.size(), overall_results.total_size());
		writeln!(output, "{lang}:").ok();
		writeln!(
			output,
			"\tFiles: {} {} ({file_pct:.1}% of total).",
			lang_stats.files(),
			utils::pluralize(lang_stats.files(), "file", "files")
		)
		.ok();
		writeln!(
			output,
			"\tLines: {} {} ({line_pct:.1}% of total).",
			lang_stats.lines(),
			utils::pluralize(lang_stats.lines(), "line", "lines")
		)
		.ok();
		writeln!(output, "\tSize: {} ({size_pct:.1}% of total).", lang_stats.size_human()).ok();
		output.push_str("\tLine breakdown:\n");
		if lang_stats.code_lines() > 0 {
			writeln!(output, "\t\tCode: {} lines ({:.1}%).", lang_stats.code_lines(), lang_stats.code_percentage())
				.ok();
		}
		if lang_stats.comment_lines() > 0 {
			writeln!(
				output,
				"\t\tComments: {} lines ({:.1}%).",
				lang_stats.comment_lines(),
				lang_stats.comment_percentage()
			)
			.ok();
		}
		if lang_stats.blank_lines() > 0 {
			writeln!(output, "\t\tBlanks: {} lines ({:.1}%).", lang_stats.blank_lines(), lang_stats.blank_percentage())
				.ok();
		}
		if lang_stats.shebang_lines() > 0 {
			writeln!(
				output,
				"\t\tShebangs: {} lines ({:.1}%).",
				lang_stats.shebang_lines(),
				lang_stats.shebang_percentage()
			)
			.ok();
		}
		if verbose {
			output.push_str(&Self::format_file_breakdown(lang_stats, overall_results));
		}
		output
	}

	fn format_file_breakdown(lang_stats: &LanguageStats, overall_results: &AnalysisResults) -> String {
		let mut output = String::from("\tFile breakdown:\n");
		let mut files: Vec<_> = lang_stats.files_list().iter().collect();
		files.sort_by_key(|b| Reverse(b.total_lines()));
		for file_stat in files {
			let file_pct = utils::percentage(file_stat.total_lines(), overall_results.total_lines());
			writeln!(
				output,
				"\t\t{}: {} lines, {} ({:.1}% of total lines).",
				file_stat.path(),
				file_stat.total_lines(),
				file_stat.size_human(),
				file_pct
			)
			.ok();
		}
		output
	}

	fn build_line_breakdown_parts(results: &AnalysisResults) -> Vec<String> {
		let mut parts = Vec::new();
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
		let mut parts = Vec::new();
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
