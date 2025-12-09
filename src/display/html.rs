use std::{io::Write, path::Path};

use anyhow::Result;
use askama::Template;

use super::{FormatterContext, OutputFormatter, ReportData, ViewOptions, report::Summary};
use crate::{analysis::AnalysisResults, display::report::FormattedLanguage};

#[derive(Template)]
#[template(path = "report.html", escape = "html")]
struct ReportTemplate<'a> {
	title: &'a str,
	summary: &'a Summary,
	totals: String,
	languages: &'a [FormattedLanguage<'a>],
	show_files: bool,
}

pub struct HtmlFormatter;

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
		let languages = report.formatted_languages(&ctx);
		Self::write_document(&report, &languages, verbose, &ctx, writer)
	}
}

impl HtmlFormatter {
	fn write_document(
		report: &ReportData,
		languages: &[FormattedLanguage],
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
			languages,
			show_files: verbose,
		};
		let rendered = template.render()?;
		writer.write_all(rendered.as_bytes())?;
		Ok(())
	}
}
