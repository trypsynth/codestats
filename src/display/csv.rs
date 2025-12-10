use std::{borrow::Cow, io::Write, path::Path};

use anyhow::Result;

use super::{FormatterContext, OutputFormatter, ReportData, ViewOptions};
use crate::{analysis::AnalysisResults, display::report::LanguageRecord};

pub struct SeparatedValuesFormatter<const DELIMITER: u8>;

impl<const DELIMITER: u8> Default for SeparatedValuesFormatter<DELIMITER> {
	fn default() -> Self {
		Self
	}
}

impl<const DELIMITER: u8> OutputFormatter for SeparatedValuesFormatter<DELIMITER> {
	fn write_output(
		&self,
		results: &AnalysisResults,
		path: &Path,
		verbose: bool,
		view_options: ViewOptions,
		writer: &mut dyn Write,
	) -> Result<()> {
		let (ctx, report) = self.prepare_report(results, path, verbose, view_options);
		if verbose {
			Self::write_verbose(&report, &ctx, writer)
		} else {
			Self::write_simple(&report.languages, &ctx, writer)
		}
	}
}

impl<const DELIMITER: u8> SeparatedValuesFormatter<DELIMITER> {
	fn write_verbose(report: &ReportData, ctx: &FormatterContext, writer: &mut dyn Write) -> Result<()> {
		Self::write_summary_section(report, ctx, writer)?;
		writer.write_all(b"\n")?;
		Self::write_language_section(&report.languages, ctx, writer)?;
		writer.write_all(b"\n")?;
		Self::write_files_sections(&report.languages, ctx, writer)?;
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

	fn write_language_section(
		languages: &[LanguageRecord],
		ctx: &FormatterContext,
		output: &mut dyn Write,
	) -> Result<()> {
		output.write_all(b"Language breakdown:\n")?;
		Self::write_language_header(output)?;
		for lang in languages {
			Self::write_language_row(lang, ctx, output)?;
		}
		output.write_all(b"\n")?;
		Ok(())
	}

	fn write_files_sections(
		languages: &[LanguageRecord],
		ctx: &FormatterContext,
		output: &mut dyn Write,
	) -> Result<()> {
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
						&file_stat.format_total_lines(ctx),
						&file_stat.format_code_lines(ctx),
						&file_stat.format_comment_lines(ctx),
						&file_stat.format_blank_lines(ctx),
						&file_stat.format_shebang_lines(ctx),
						&file_stat.format_size(ctx),
						&file_stat.size_human,
					],
				)?;
			}
			output.write_all(b"\n")?;
		}
		Ok(())
	}

	fn write_simple(languages: &[LanguageRecord], ctx: &FormatterContext, output: &mut dyn Write) -> Result<()> {
		Self::write_language_header(output)?;
		for lang in languages {
			Self::write_language_row(lang, ctx, output)?;
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

	fn write_language_row(lang: &LanguageRecord, ctx: &FormatterContext, output: &mut dyn Write) -> Result<()> {
		Self::write_record(
			output,
			&[
				lang.name,
				&lang.format_files(ctx),
				&lang.format_lines(ctx),
				&lang.format_code_lines(ctx),
				&lang.format_comment_lines(ctx),
				&lang.format_blank_lines(ctx),
				&lang.format_shebang_lines(ctx),
				&lang.format_size(ctx),
				&lang.size_human,
				&lang.format_code_percentage(ctx),
				&lang.format_comment_percentage(ctx),
				&lang.format_blank_percentage(ctx),
				&lang.format_shebang_percentage(ctx),
			],
		)?;
		Ok(())
	}

	fn write_record(output: &mut dyn Write, fields: &[&str]) -> Result<()> {
		for (idx, field) in fields.iter().enumerate() {
			if idx > 0 {
				output.write_all(&[DELIMITER])?;
			}
			Self::write_field(output, field)?;
		}
		output.write_all(b"\n")?;
		Ok(())
	}

	fn write_field(output: &mut dyn Write, field: &str) -> Result<()> {
		output.write_all(Self::escape_field(field).as_bytes())?;
		Ok(())
	}

	fn escape_field(field: &str) -> Cow<'_, str> {
		let delimiter = char::from(DELIMITER);
		let needs_quotes =
			field.contains(delimiter) || field.contains('"') || field.contains('\n') || field.contains('\r');
		if !needs_quotes {
			return Cow::Borrowed(field);
		}
		let escaped = field.replace('"', "\"\"");
		Cow::Owned(format!("\"{escaped}\""))
	}
}

pub type CsvFormatter = SeparatedValuesFormatter<b','>;
