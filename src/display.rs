use std::path::Path;

use human_bytes::human_bytes;

use crate::{
	analysis::{AnalysisResults, LanguageStats},
	utils,
};

/// Formats and displays analysis results
pub struct ResultFormatter;

impl ResultFormatter {
	pub fn print_summary(results: &AnalysisResults, path: &Path, verbose: bool) {
		Self::print_overview(results, path);
		if results.language_stats().is_empty() {
			println!("No recognized programming languages found.");
			return;
		}
		Self::print_language_breakdown(results, verbose);
	}

	pub fn print_overview(results: &AnalysisResults, path: &Path) {
		println!(
			"Codestats for {}: {} {}, {} total {}, {} total size.",
			path.display(),
			results.total_files(),
			utils::pluralize(results.total_files(), "file", "files"),
			results.total_lines(),
			utils::pluralize(results.total_lines(), "line", "lines"),
			human_bytes(results.total_size() as f64)
		);
		let line_breakdown_parts = Self::build_line_breakdown_parts(results);
		if !line_breakdown_parts.is_empty() {
			println!("Line breakdown: {}", line_breakdown_parts.join(", "));
		}
		let percentage_parts = Self::build_percentage_parts(results);
		if !percentage_parts.is_empty() {
			println!("Percentages: {}", percentage_parts.join(", "));
		}
	}

	pub fn print_language_breakdown(results: &AnalysisResults, verbose: bool) {
		println!("Language breakdown:");
		for (lang, lang_stats) in results.languages_by_lines() {
			Self::print_language_stats(lang, lang_stats, results, verbose);
		}
	}

	fn print_language_stats(lang: &str, lang_stats: &LanguageStats, overall_results: &AnalysisResults, verbose: bool) {
		let file_pct = utils::percentage(lang_stats.files(), overall_results.total_files());
		let line_pct = utils::percentage(lang_stats.lines(), overall_results.total_lines());
		let size_pct = utils::percentage(lang_stats.size(), overall_results.total_size());
		println!("{lang}:");
		println!(
			"\tFiles: {} {} ({file_pct:.1}% of total).",
			lang_stats.files(),
			utils::pluralize(lang_stats.files(), "file", "files")
		);
		println!(
			"\tLines: {} {} ({line_pct:.1}% of total).",
			lang_stats.lines(),
			utils::pluralize(lang_stats.lines(), "line", "lines")
		);
		println!("\tSize: {} ({size_pct:.1}% of total).", human_bytes(lang_stats.size() as f64));
		println!("\tLine breakdown:");
		if lang_stats.code_lines() > 0 {
			println!("\t\tCode: {} lines ({:.1}%).", lang_stats.code_lines(), lang_stats.code_percentage());
		}
		if lang_stats.comment_lines() > 0 {
			println!("\t\tComments: {} lines ({:.1}%).", lang_stats.comment_lines(), lang_stats.comment_percentage());
		}
		if lang_stats.blank_lines() > 0 {
			println!("\t\tBlanks: {} lines ({:.1}%).", lang_stats.blank_lines(), lang_stats.blank_percentage());
		}
		if lang_stats.shebang_lines() > 0 {
			println!("\t\tShebangs: {} lines ({:.1}%).", lang_stats.shebang_lines(), lang_stats.shebang_percentage());
		}
		if verbose {
			Self::print_file_breakdown(lang_stats, overall_results);
		}
	}

	fn print_file_breakdown(lang_stats: &LanguageStats, overall_results: &AnalysisResults) {
		println!("\tFile breakdown:");
		let mut files: Vec<_> = lang_stats.files_list().iter().collect();
		files.sort_by(|a, b| b.total_lines().cmp(&a.total_lines()));
		for file_stat in files {
			let file_pct = utils::percentage(file_stat.total_lines(), overall_results.total_lines());
			println!("\t\t{}: {} lines ({:.1}% of total).", file_stat.path(), file_stat.total_lines(), file_pct);
		}
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
