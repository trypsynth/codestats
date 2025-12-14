use std::{io::Write, path::Path};

use anyhow::Result;
use askama::{Result as AskamaResult, Template, Values};

use super::{FormatterContext, OutputFormatter, ReportData, ViewOptions};
use crate::{
	analysis::AnalysisResults,
	display::report::{LanguageRecord, Summary},
};

#[derive(Template)]
#[template(path = "report.html", escape = "html")]
struct ReportTemplate<'a> {
	title: &'a str,
	summary: &'a Summary,
	totals: String,
	languages: &'a [LanguageRecord<'a>],
	ctx: &'a FormatterContext,
	show_files: bool,
}

pub struct HtmlFormatter;

#[expect(clippy::unnecessary_wraps, clippy::trivially_copy_pass_by_ref)]
pub fn fmt_number(value: &u64, _values: &dyn Values, ctx: &FormatterContext) -> AskamaResult<String> {
	Ok(ctx.number(*value))
}

#[expect(clippy::unnecessary_wraps, clippy::trivially_copy_pass_by_ref)]
pub fn fmt_percent(value: &f64, _values: &dyn Values, ctx: &FormatterContext) -> AskamaResult<String> {
	Ok(ctx.percent(*value))
}

mod filters {
	pub use super::{fmt_number, fmt_percent};
}

impl OutputFormatter for HtmlFormatter {
	fn write_output(
		&self,
		results: &AnalysisResults,
		path: &Path,
		verbose: bool,
		view_options: ViewOptions,
		writer: &mut dyn Write,
	) -> Result<()> {
		let (ctx, report) = self.prepare_report(results, path, verbose, view_options);
		Self::write_document(&report, verbose, &ctx, writer)
	}
}

impl HtmlFormatter {
	fn write_document(
		report: &ReportData,
		verbose: bool,
		ctx: &FormatterContext,
		writer: &mut dyn Write,
	) -> Result<()> {
		let parts = report.summary.percentage_parts(ctx);
		let totals = if parts.is_empty() { String::new() } else { parts.join(", ") };
		let template = ReportTemplate {
			title: &report.analysis_path,
			summary: &report.summary,
			totals,
			languages: &report.languages,
			ctx,
			show_files: verbose,
		};
		let rendered = template.render()?;
		writer.write_all(rendered.as_bytes())?;
		Ok(())
	}
}
