use std::{cmp::Reverse, io::Write, path::Path};

use anyhow::Result;

use super::OutputFormatter;
use crate::analysis::{AnalysisResults, LanguageStats};

/// CSV formatter
pub struct CsvFormatter;

impl OutputFormatter for CsvFormatter {
	fn write_output(
		&self,
		results: &AnalysisResults,
		path: &Path,
		verbose: bool,
		writer: &mut dyn Write,
	) -> Result<()> {
		if verbose { Self::write_verbose(results, path, writer) } else { Self::write_simple(results, writer) }
	}
}

impl CsvFormatter {
	fn write_verbose(results: &AnalysisResults, path: &Path, writer: &mut dyn Write) -> Result<()> {
		let languages = results.languages_by_lines();
		Self::write_summary_section(results, path, writer)?;
		writer.write_all(b"\n")?;
		Self::write_language_section(&languages, writer)?;
		writer.write_all(b"\n")?;
		Self::write_files_sections(&languages, writer)?;
		Ok(())
	}

	fn write_summary_section(results: &AnalysisResults, path: &Path, output: &mut dyn Write) -> Result<()> {
		output.write_all(b"Summary:\n")?;
		Self::write_record(output, &["metric", "value", "percentage", "human_readable"])?;
		let path_display = path.display().to_string();
		let total_files = results.total_files().to_string();
		let total_lines = results.total_lines().to_string();
		let code_lines = results.total_code_lines().to_string();
		let code_pct = format!("{:.2}", results.code_percentage());
		let comment_lines = results.total_comment_lines().to_string();
		let comment_pct = format!("{:.2}", results.comment_percentage());
		let blank_lines = results.total_blank_lines().to_string();
		let blank_pct = format!("{:.2}", results.blank_percentage());
		let shebang_lines = results.total_shebang_lines().to_string();
		let shebang_pct = format!("{:.2}", results.shebang_percentage());
		let total_size = results.total_size().to_string();
		let total_size_human = results.total_size_human();
		Self::write_record(output, &["Analysis Path", path_display.as_str(), "", ""])?;
		Self::write_record(output, &["Total Files", total_files.as_str(), "100.00", ""])?;
		Self::write_record(output, &["Total Lines", total_lines.as_str(), "100.00", ""])?;
		Self::write_record(output, &["Code Lines", code_lines.as_str(), code_pct.as_str(), ""])?;
		Self::write_record(output, &["Comment Lines", comment_lines.as_str(), comment_pct.as_str(), ""])?;
		Self::write_record(output, &["Blank Lines", blank_lines.as_str(), blank_pct.as_str(), ""])?;
		Self::write_record(output, &["Shebang Lines", shebang_lines.as_str(), shebang_pct.as_str(), ""])?;
		Self::write_record(output, &["Total Size", total_size.as_str(), "100.00", total_size_human.as_str()])?;
		Ok(())
	}

	fn write_language_section(languages: &[(&'static str, &LanguageStats)], output: &mut dyn Write) -> Result<()> {
		output.write_all(b"Language breakdown:\n")?;
		Self::write_language_header(output)?;
		for (lang, stats) in languages {
			Self::write_language_row(lang, stats, output)?;
		}
		output.write_all(b"\n")?;
		Ok(())
	}

	fn write_files_sections(languages: &[(&'static str, &LanguageStats)], output: &mut dyn Write) -> Result<()> {
		for (lang_name, lang_stats) in languages {
			writeln!(output, "{lang_name} files:")?;
			Self::write_record(
				output,
				&[
					"file_path",
					"total_lines",
					"code_lines",
					"comment_lines",
					"blank_lines",
					"shebang_lines",
					"size",
					"size_human",
				],
			)?;
			let mut files: Vec<_> = lang_stats.files_list().iter().collect();
			files.sort_by_key(|f| Reverse(f.total_lines()));
			for file_stat in files {
				let total_lines = file_stat.total_lines().to_string();
				let code_lines = file_stat.code_lines().to_string();
				let comment_lines = file_stat.comment_lines().to_string();
				let blank_lines = file_stat.blank_lines().to_string();
				let shebang_lines = file_stat.shebang_lines().to_string();
				let size = file_stat.size().to_string();
				let size_human = file_stat.size_human();
				Self::write_record(
					output,
					&[
						file_stat.path(),
						total_lines.as_str(),
						code_lines.as_str(),
						comment_lines.as_str(),
						blank_lines.as_str(),
						shebang_lines.as_str(),
						size.as_str(),
						size_human.as_str(),
					],
				)?;
			}
			output.write_all(b"\n")?;
		}
		Ok(())
	}

	fn write_simple(results: &AnalysisResults, output: &mut dyn Write) -> Result<()> {
		let languages = results.languages_by_lines();
		Self::write_language_header(output)?;
		for (lang, stats) in languages {
			Self::write_language_row(lang, stats, output)?;
		}
		Ok(())
	}

	fn write_language_header(output: &mut dyn Write) -> Result<()> {
		Self::write_record(
			output,
			&[
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
			],
		)?;
		Ok(())
	}

	fn write_language_row(lang: &str, stats: &LanguageStats, output: &mut dyn Write) -> Result<()> {
		let files = stats.files().to_string();
		let lines = stats.lines().to_string();
		let code_lines = stats.code_lines().to_string();
		let comment_lines = stats.comment_lines().to_string();
		let blank_lines = stats.blank_lines().to_string();
		let shebang_lines = stats.shebang_lines().to_string();
		let size = stats.size().to_string();
		let code_pct = format!("{:.2}", stats.code_percentage());
		let comment_pct = format!("{:.2}", stats.comment_percentage());
		let blank_pct = format!("{:.2}", stats.blank_percentage());
		let shebang_pct = format!("{:.2}", stats.shebang_percentage());
		let size_human = stats.size_human();
		Self::write_record(
			output,
			&[
				lang,
				files.as_str(),
				lines.as_str(),
				code_lines.as_str(),
				comment_lines.as_str(),
				blank_lines.as_str(),
				shebang_lines.as_str(),
				size.as_str(),
				size_human.as_str(),
				code_pct.as_str(),
				comment_pct.as_str(),
				blank_pct.as_str(),
				shebang_pct.as_str(),
			],
		)?;
		Ok(())
	}

	fn write_record(output: &mut dyn Write, fields: &[&str]) -> Result<()> {
		for (idx, field) in fields.iter().enumerate() {
			if idx > 0 {
				output.write_all(b",")?;
			}
			Self::write_csv_field(output, field)?;
		}
		output.write_all(b"\n")?;
		Ok(())
	}

	fn write_csv_field(output: &mut dyn Write, field: &str) -> Result<()> {
		let needs_quotes = field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r');
		if needs_quotes {
			output.write_all(b"\"")?;
		}
		for ch in field.chars() {
			if ch == '"' {
				output.write_all(b"\"\"")?;
			} else {
				let mut buf = [0; 4];
				let s = ch.encode_utf8(&mut buf);
				output.write_all(s.as_bytes())?;
			}
		}
		if needs_quotes {
			output.write_all(b"\"")?;
		}
		Ok(())
	}
}
