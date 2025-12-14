use std::{io::Write, path::Path};

use anyhow::Result;
use askama::{Result as AskamaResult, Template, Values};

use super::{FormatterContext, OutputFormatter, ReportData, ViewOptions};
use crate::{
	analysis::AnalysisResults,
	display::report::{LanguageRecord, Summary},
};

/// Escape Markdown table cells by escaping the pipe separator.
#[expect(clippy::unnecessary_wraps)]
pub fn md_escape(value: &str, _values: &dyn Values) -> AskamaResult<String> {
	Ok(value.replace('|', "\\|"))
}

#[expect(clippy::unnecessary_wraps, clippy::trivially_copy_pass_by_ref)]
pub fn fmt_number(value: &u64, _values: &dyn Values, ctx: &FormatterContext) -> AskamaResult<String> {
	Ok(ctx.number(*value))
}

#[expect(clippy::unnecessary_wraps, clippy::trivially_copy_pass_by_ref)]
pub fn fmt_percent(value: &f64, _values: &dyn Values, ctx: &FormatterContext) -> AskamaResult<String> {
	Ok(ctx.percent(*value))
}

mod filters {
	pub use super::{fmt_number, fmt_percent, md_escape};
}

#[derive(Template)]
#[template(path = "report.md", escape = "none")]
struct MarkdownTemplate<'a> {
	title: &'a str,
	summary: &'a Summary,
	ctx: &'a FormatterContext,
	line_breakdown: Vec<String>,
	totals: Vec<String>,
	languages: &'a [LanguageRecord<'a>],
	show_files: bool,
}

pub struct MarkdownFormatter;

impl OutputFormatter for MarkdownFormatter {
	fn write_output(
		&self,
		results: &AnalysisResults,
		path: &Path,
		verbose: bool,
		view_options: ViewOptions,
		writer: &mut dyn Write,
	) -> Result<()> {
		let (ctx, report) = self.prepare_report(results, path, verbose, view_options);
		Self::write_markdown(&report, verbose, &ctx, writer)
	}
}

impl MarkdownFormatter {
	fn write_markdown(
		report: &ReportData,
		verbose: bool,
		ctx: &FormatterContext,
		writer: &mut dyn Write,
	) -> Result<()> {
		let summary = &report.summary;
		let line_breakdown = summary.line_breakdown_parts(false, ctx);
		let totals = summary.percentage_parts(ctx);
		let template = MarkdownTemplate {
			title: &report.analysis_path,
			summary,
			ctx,
			line_breakdown,
			totals,
			languages: &report.languages,
			show_files: verbose,
		};
		let rendered = template.render()?;
		writer.write_all(rendered.as_bytes())?;
		Ok(())
	}
}
