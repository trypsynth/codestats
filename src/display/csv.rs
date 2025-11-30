use std::{io::Write, path::Path};

use anyhow::Result;

use super::{FormatterContext, OutputFormatter, ReportData, ViewOptions};
use crate::{analysis::AnalysisResults, display::report::FormattedLanguage, utils};

pub struct CsvFormatter;

impl OutputFormatter for CsvFormatter {
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
		if verbose {
			Self::write_verbose(&report, &languages, &ctx, writer)
		} else {
			Self::write_simple(&languages, writer)
		}
	}
}

impl CsvFormatter {
	fn write_verbose(
		report: &ReportData,
		languages: &[FormattedLanguage],
		ctx: &FormatterContext,
		writer: &mut dyn Write,
	) -> Result<()> {
		Self::write_summary_section(report, ctx, writer)?;
		writer.write_all(b"\n")?;
		Self::write_language_section(languages, writer)?;
		writer.write_all(b"\n")?;
		Self::write_files_sections(languages, writer)?;
		Ok(())
	}

	fn write_summary_section(report: &ReportData, ctx: &FormatterContext, output: &mut dyn Write) -> Result<()> {
		output.write_all(b"Summary:\n")?;
		Self::write_record(output, &["metric", "value", "percentage", "human_readable"])?;
		let summary = &report.summary;
		let total_files = ctx.number(summary.total_files);
		let total_lines = ctx.number(summary.total_lines);
		let code_lines = ctx.number(summary.total_code_lines);
		let code_pct = ctx.percent(summary.code_percentage);
		let comment_lines = ctx.number(summary.total_comment_lines);
		let comment_pct = ctx.percent(summary.comment_percentage);
		let blank_lines = ctx.number(summary.total_blank_lines);
		let blank_pct = ctx.percent(summary.blank_percentage);
		let shebang_lines = ctx.number(summary.total_shebang_lines);
		let shebang_pct = ctx.percent(summary.shebang_percentage);
		let total_size = ctx.number(summary.total_size);
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
				Self::write_record(
					output,
					&[
						file_stat.path,
						file_stat.total_lines.as_str(),
						file_stat.code_lines.as_str(),
						file_stat.comment_lines.as_str(),
						file_stat.blank_lines.as_str(),
						file_stat.shebang_lines.as_str(),
						file_stat.size.as_str(),
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
		Self::write_record(
			output,
			&[
				lang.name,
				lang.files.as_str(),
				lang.lines.as_str(),
				lang.code_lines.as_str(),
				lang.comment_lines.as_str(),
				lang.blank_lines.as_str(),
				lang.shebang_lines.as_str(),
				lang.size.as_str(),
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
		output.write_all(utils::escape_csv_field(field).as_bytes())?;
		Ok(())
	}
}
