use std::{io::Write, path::Path};

use anyhow::Result;
use askama::Template;

use super::{FormatterContext, OutputFormatter, ReportData, ViewOptions};
use crate::{analysis::AnalysisResults, display::report::FormattedLanguage, filters};

#[derive(Template)]
#[template(path = "report.md", escape = "none")]
struct MarkdownTemplate<'a> {
	title: &'a str,
	total_files: String,
	total_lines: String,
	total_size_human: &'a str,
	line_breakdown: Vec<String>,
	totals: Vec<String>,
	languages: &'a [FormattedLanguage<'a>],
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
		let ctx = FormatterContext::new(view_options);
		let report = ReportData::from_results(results, path, verbose, &ctx);
		let languages = report.formatted_languages(&ctx);
		Self::write_markdown(&report, &languages, verbose, &ctx, writer)
	}
}

impl MarkdownFormatter {
	fn write_markdown(
		report: &ReportData,
		languages: &[FormattedLanguage],
		verbose: bool,
		ctx: &FormatterContext,
		writer: &mut dyn Write,
	) -> Result<()> {
		let summary = &report.summary;
		let line_breakdown = summary.line_breakdown_parts(false, ctx);
		let totals = summary.percentage_parts(ctx);
		let template = MarkdownTemplate {
			title: &report.analysis_path,
			total_files: ctx.number(summary.total_files),
			total_lines: ctx.number(summary.total_lines),
			total_size_human: &summary.total_size_human,
			line_breakdown,
			totals,
			languages,
			show_files: verbose,
		};
		let rendered = template.render()?;
		writer.write_all(rendered.as_bytes())?;
		Ok(())
	}
}
