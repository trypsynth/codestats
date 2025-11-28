use std::{io::Write, path::Path};

use anyhow::Result;
use serde::Serialize;
use serde_json::to_writer_pretty;

use super::OutputFormatter;
use crate::analysis::AnalysisResults;

pub struct JsonFormatter;

impl OutputFormatter for JsonFormatter {
	fn write_output(
		&self,
		results: &AnalysisResults,
		path: &Path,
		verbose: bool,
		writer: &mut dyn Write,
	) -> Result<()> {
		let payload = JsonOutput::from_results(results, path, verbose);
		to_writer_pretty(writer, &payload)?;
		Ok(())
	}
}

#[derive(Serialize)]
struct JsonOutput<'a> {
	analysis_path: String,
	summary: Summary,
	languages: Vec<LanguageRecord<'a>>,
}

impl<'a> JsonOutput<'a> {
	fn from_results(results: &'a AnalysisResults, path: &Path, verbose: bool) -> Self {
		let summary = Summary::from_results(results);
		let languages = LanguageRecord::from_results(results, verbose);
		Self { analysis_path: path.display().to_string(), summary, languages }
	}
}

#[derive(Serialize)]
struct Summary {
	total_files: u64,
	total_lines: u64,
	total_code_lines: u64,
	total_comment_lines: u64,
	total_blank_lines: u64,
	total_shebang_lines: u64,
	total_size: u64,
	total_size_human: String,
	code_percentage: f64,
	comment_percentage: f64,
	blank_percentage: f64,
	shebang_percentage: f64,
}

impl Summary {
	fn from_results(results: &AnalysisResults) -> Self {
		Self {
			total_files: results.total_files(),
			total_lines: results.total_lines(),
			total_code_lines: results.total_code_lines(),
			total_comment_lines: results.total_comment_lines(),
			total_blank_lines: results.total_blank_lines(),
			total_shebang_lines: results.total_shebang_lines(),
			total_size: results.total_size(),
			total_size_human: results.total_size_human(),
			code_percentage: results.code_percentage(),
			comment_percentage: results.comment_percentage(),
			blank_percentage: results.blank_percentage(),
			shebang_percentage: results.shebang_percentage(),
		}
	}
}

#[derive(Serialize)]
struct LanguageRecord<'a> {
	name: &'a str,
	files: u64,
	lines: u64,
	code_lines: u64,
	comment_lines: u64,
	blank_lines: u64,
	shebang_lines: u64,
	size: u64,
	size_human: String,
	code_percentage: f64,
	comment_percentage: f64,
	blank_percentage: f64,
	shebang_percentage: f64,
	#[serde(skip_serializing_if = "Option::is_none")]
	files_detail: Option<Vec<FileRecord<'a>>>,
}

impl<'a> LanguageRecord<'a> {
	fn from_results(results: &'a AnalysisResults, verbose: bool) -> Vec<Self> {
		results
			.languages_by_lines()
			.into_iter()
			.map(|(name, stats)| {
				let files_detail = verbose.then(|| {
					stats
						.files_list()
						.iter()
						.map(|file| FileRecord {
							path: file.path(),
							total_lines: file.total_lines(),
							code_lines: file.code_lines(),
							comment_lines: file.comment_lines(),
							blank_lines: file.blank_lines(),
							shebang_lines: file.shebang_lines(),
							size: file.size(),
							size_human: file.size_human(),
						})
						.collect()
				});
				LanguageRecord {
					name,
					files: stats.files(),
					lines: stats.lines(),
					code_lines: stats.code_lines(),
					comment_lines: stats.comment_lines(),
					blank_lines: stats.blank_lines(),
					shebang_lines: stats.shebang_lines(),
					size: stats.size(),
					size_human: stats.size_human(),
					code_percentage: stats.code_percentage(),
					comment_percentage: stats.comment_percentage(),
					blank_percentage: stats.blank_percentage(),
					shebang_percentage: stats.shebang_percentage(),
					files_detail,
				}
			})
			.collect()
	}
}

#[derive(Serialize)]
struct FileRecord<'a> {
	path: &'a str,
	total_lines: u64,
	code_lines: u64,
	comment_lines: u64,
	blank_lines: u64,
	shebang_lines: u64,
	size: u64,
	size_human: String,
}
