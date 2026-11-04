use std::{io::Write, path::PathBuf};

use anyhow::Result;
use askama::Template;

use super::{FormatterContext, OutputFormatter, ReportData, Verbosity, ViewOptions};
use crate::{
	analysis::AnalysisResults,
	display::report::{DirRecord, EmbeddedRow, LanguageRecord, Summary},
};

#[derive(Template)]
#[template(path = "report.html", escape = "html")]
struct ReportTemplate<'a> {
	title: &'a str,
	summary: &'a Summary,
	totals: String,
	languages: &'a [LanguageRecord<'a>],
	directories: &'a [DirRecord<'a>],
	/// Whether to show the language or directory breakdown at all (`-q` shows only the summary).
	show_breakdown: bool,
	by_dir: bool,
	embedded: Vec<EmbeddedRow<'a>>,
	ctx: &'a FormatterContext,
	show_files: bool,
	hidden_note: Option<String>,
}

pub struct HtmlFormatter;

mod filters {
	pub use crate::display::template_filters::{fmt_float, fmt_number, fmt_percent};
}

impl OutputFormatter for HtmlFormatter {
	fn write_output(
		&self,
		results: &AnalysisResults,
		paths: &[PathBuf],
		view_options: ViewOptions,
		writer: &mut dyn Write,
	) -> Result<()> {
		let (ctx, report) = self.prepare_report(results, paths, view_options);
		Self::write_document(&report, view_options.verbosity, &ctx, writer)
	}
}

impl HtmlFormatter {
	fn write_document(
		report: &ReportData,
		verbosity: Verbosity,
		ctx: &FormatterContext,
		writer: &mut dyn Write,
	) -> Result<()> {
		let parts = report.summary.percentage_parts(ctx);
		let totals = parts.join(", ");
		let template = ReportTemplate {
			title: report.title(),
			summary: &report.summary,
			totals,
			languages: &report.languages,
			directories: &report.directories,
			show_breakdown: verbosity > Verbosity::Summary,
			by_dir: ctx.options.by_dir,
			embedded: report.embedded_rows(),
			ctx,
			show_files: verbosity == Verbosity::Verbose,
			hidden_note: report.hidden_note(ctx.options.by_dir),
		};
		let rendered = template.render()?;
		writer.write_all(rendered.as_bytes())?;
		Ok(())
	}
}
