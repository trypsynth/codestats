use std::{io::Write, path::Path};

use anyhow::Result;

use super::{OutputFormatter, ReportData};
use crate::{analysis::AnalysisResults, display::report::FormattedLanguage};

pub struct CsvFormatter;

impl OutputFormatter for CsvFormatter {
	fn write_output(
		&self,
		results: &AnalysisResults,
		path: &Path,
		verbose: bool,
		writer: &mut dyn Write,
	) -> Result<()> {
		let report = ReportData::from_results(results, path, verbose);
		let languages = report.formatted_languages();
		if verbose { Self::write_verbose(&report, &languages, writer) } else { Self::write_simple(&languages, writer) }
	}
}

impl CsvFormatter {
	fn write_verbose(report: &ReportData, languages: &[FormattedLanguage], writer: &mut dyn Write) -> Result<()> {
		Self::write_summary_section(report, writer)?;
		writer.write_all(b"\n")?;
		Self::write_language_section(languages, writer)?;
		writer.write_all(b"\n")?;
		Self::write_files_sections(languages, writer)?;
		Ok(())
	}

	fn write_summary_section(report: &ReportData, output: &mut dyn Write) -> Result<()> {
		output.write_all(b"Summary:\n")?;
		Self::write_record(output, &["metric", "value", "percentage", "human_readable"])?;
		let summary = &report.summary;
		let total_files = summary.total_files.to_string();
		let total_lines = summary.total_lines.to_string();
		let code_lines = summary.total_code_lines.to_string();
		let code_pct = format!("{:.2}", summary.code_percentage);
		let comment_lines = summary.total_comment_lines.to_string();
		let comment_pct = format!("{:.2}", summary.comment_percentage);
		let blank_lines = summary.total_blank_lines.to_string();
		let blank_pct = format!("{:.2}", summary.blank_percentage);
		let shebang_lines = summary.total_shebang_lines.to_string();
		let shebang_pct = format!("{:.2}", summary.shebang_percentage);
		let total_size = summary.total_size.to_string();
		let total_size_human = &summary.total_size_human;
		Self::write_record(output, &["Analysis Path", report.analysis_path.as_str(), "", ""])?;
		Self::write_record(output, &["Total Files", total_files.as_str(), "100.00", ""])?;
		Self::write_record(output, &["Total Lines", total_lines.as_str(), "100.00", ""])?;
		Self::write_record(output, &["Code Lines", code_lines.as_str(), code_pct.as_str(), ""])?;
		Self::write_record(output, &["Comment Lines", comment_lines.as_str(), comment_pct.as_str(), ""])?;
		Self::write_record(output, &["Blank Lines", blank_lines.as_str(), blank_pct.as_str(), ""])?;
		Self::write_record(output, &["Shebang Lines", shebang_lines.as_str(), shebang_pct.as_str(), ""])?;
		Self::write_record(output, &["Total Size", total_size.as_str(), "100.00", total_size_human.as_str()])?;
		Ok(())
	}

	fn write_language_section(languages: &[FormattedLanguage], output: &mut dyn Write) -> Result<()> {
		output.write_all(b"Language breakdown:\n")?;
		Self::write_language_header(output)?;
		for lang in languages {
			Self::write_language_row(lang, output)?;
		}
		output.write_all(b"\n")?;
		Ok(())
	}

	fn write_files_sections(languages: &[FormattedLanguage], output: &mut dyn Write) -> Result<()> {
		for language in languages {
			let Some(files) = &language.files_detail else {
				continue;
			};
			writeln!(output, "{} files:", language.name)?;
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
			for file_stat in files {
				let total_lines = file_stat.total_lines.to_string();
				let code_lines = file_stat.code_lines.to_string();
				let comment_lines = file_stat.comment_lines.to_string();
				let blank_lines = file_stat.blank_lines.to_string();
				let shebang_lines = file_stat.shebang_lines.to_string();
				let size = file_stat.size.to_string();
				Self::write_record(
					output,
					&[
						file_stat.path,
						total_lines.as_str(),
						code_lines.as_str(),
						comment_lines.as_str(),
						blank_lines.as_str(),
						shebang_lines.as_str(),
						size.as_str(),
						file_stat.size_human,
					],
				)?;
			}
			output.write_all(b"\n")?;
		}
		Ok(())
	}

	fn write_simple(languages: &[FormattedLanguage], output: &mut dyn Write) -> Result<()> {
		Self::write_language_header(output)?;
		for lang in languages {
			Self::write_language_row(lang, output)?;
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

	fn write_language_row(lang: &FormattedLanguage, output: &mut dyn Write) -> Result<()> {
		let files = lang.files.to_string();
		let lines = lang.lines.to_string();
		let code_lines = lang.code_lines.to_string();
		let comment_lines = lang.comment_lines.to_string();
		let blank_lines = lang.blank_lines.to_string();
		let shebang_lines = lang.shebang_lines.to_string();
		let size = lang.size.to_string();
		Self::write_record(
			output,
			&[
				lang.name,
				files.as_str(),
				lines.as_str(),
				code_lines.as_str(),
				comment_lines.as_str(),
				blank_lines.as_str(),
				shebang_lines.as_str(),
				size.as_str(),
				lang.size_human,
				lang.code_percentage.as_str(),
				lang.comment_percentage.as_str(),
				lang.blank_percentage.as_str(),
				lang.shebang_percentage.as_str(),
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
