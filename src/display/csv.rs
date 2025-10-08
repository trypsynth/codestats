use std::{cmp::Reverse, fmt::Write, path::Path};

use anyhow::Result;
use csv::Writer;

use super::OutputFormatter;
use crate::analysis::AnalysisResults;

/// CSV formatter
pub struct CsvFormatter;

impl OutputFormatter for CsvFormatter {
	#[allow(clippy::too_many_lines)]
	fn format(&self, results: &AnalysisResults, path: &Path, verbose: bool) -> Result<String> {
		let mut output = String::new();
		if verbose {
			let mut wtr = Writer::from_writer(Vec::new());
			wtr.write_record(["metric", "value", "percentage", "human_readable"])?;
			wtr.write_record(["Analysis Path", &path.display().to_string(), "", ""])?;
			wtr.write_record(["Total Files", &results.total_files().to_string(), "100.00", ""])?;
			wtr.write_record(["Total Lines", &results.total_lines().to_string(), "100.00", ""])?;
			wtr.write_record([
				"Code Lines",
				&results.total_code_lines().to_string(),
				&format!("{:.2}", results.code_percentage()),
				"",
			])?;
			wtr.write_record([
				"Comment Lines",
				&results.total_comment_lines().to_string(),
				&format!("{:.2}", results.comment_percentage()),
				"",
			])?;
			wtr.write_record([
				"Blank Lines",
				&results.total_blank_lines().to_string(),
				&format!("{:.2}", results.blank_percentage()),
				"",
			])?;
			wtr.write_record([
				"Shebang Lines",
				&results.total_shebang_lines().to_string(),
				&format!("{:.2}", results.shebang_percentage()),
				"",
			])?;
			wtr.write_record(["Total Size", &results.total_size().to_string(), "100.00", results.total_size_human()])?;
			let summary_data = wtr.into_inner()?;
			output.push_str("Summary:\n");
			output.push_str(&String::from_utf8(summary_data)?);
			output.push('\n');
			let mut wtr = Writer::from_writer(Vec::new());
			wtr.write_record([
				"language",
				"files",
				"lines",
				"code_lines",
				"comment_lines",
				"blank_lines",
				"shebang_lines",
				"size",
				"size_human",
				"code_percentage",
				"comment_percentage",
				"blank_percentage",
				"shebang_percentage",
			])?;
			for (lang, stats) in results.languages_by_lines() {
				wtr.write_record([
					lang,
					&stats.files().to_string(),
					&stats.lines().to_string(),
					&stats.code_lines().to_string(),
					&stats.comment_lines().to_string(),
					&stats.blank_lines().to_string(),
					&stats.shebang_lines().to_string(),
					&stats.size().to_string(),
					stats.size_human(),
					&format!("{:.2}", stats.code_percentage()),
					&format!("{:.2}", stats.comment_percentage()),
					&format!("{:.2}", stats.blank_percentage()),
					&format!("{:.2}", stats.shebang_percentage()),
				])?;
			}
			let lang_data = wtr.into_inner()?;
			output.push_str("Language breakdown:\n");
			output.push_str(&String::from_utf8(lang_data)?);
			output.push('\n');
			for (lang_name, lang_stats) in results.languages_by_lines() {
				let mut wtr = csv::Writer::from_writer(Vec::new());
				wtr.write_record([
					"file_path",
					"total_lines",
					"code_lines",
					"comment_lines",
					"blank_lines",
					"shebang_lines",
					"size",
					"size_human",
				])?;
				let mut files: Vec<_> = lang_stats.files_list().iter().collect();
				files.sort_by_key(|f| Reverse(f.total_lines()));
				for file_stat in files {
					wtr.write_record([
						file_stat.path(),
						&file_stat.total_lines().to_string(),
						&file_stat.code_lines().to_string(),
						&file_stat.comment_lines().to_string(),
						&file_stat.blank_lines().to_string(),
						&file_stat.shebang_lines().to_string(),
						&file_stat.size().to_string(),
						file_stat.size_human(),
					])?;
				}
				let file_data = wtr.into_inner()?;
				writeln!(output, "{lang_name} files:")?;
				output.push_str(&String::from_utf8(file_data)?);
				output.push('\n');
			}
		} else {
			let mut wtr = Writer::from_writer(Vec::new());
			wtr.write_record([
				"language",
				"files",
				"lines",
				"code_lines",
				"comment_lines",
				"blank_lines",
				"shebang_lines",
				"size",
				"size_human",
				"code_percentage",
				"comment_percentage",
				"blank_percentage",
				"shebang_percentage",
			])?;
			for (lang, stats) in results.languages_by_lines() {
				wtr.write_record([
					lang,
					&stats.files().to_string(),
					&stats.lines().to_string(),
					&stats.code_lines().to_string(),
					&stats.comment_lines().to_string(),
					&stats.blank_lines().to_string(),
					&stats.shebang_lines().to_string(),
					&stats.size().to_string(),
					stats.size_human(),
					&format!("{:.2}", stats.code_percentage()),
					&format!("{:.2}", stats.comment_percentage()),
					&format!("{:.2}", stats.blank_percentage()),
					&format!("{:.2}", stats.shebang_percentage()),
				])?;
			}
			let data = wtr.into_inner()?;
			output.push_str(&String::from_utf8(data)?);
		}
		Ok(output)
	}
}
