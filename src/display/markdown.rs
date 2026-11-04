#![expect(clippy::unnecessary_wraps)]
// The askama `#[filter_fn]` macro generates code that triggers these lints.
#![allow(clippy::inline_always, clippy::unused_self)]

use std::{io::Write, path::PathBuf};

use anyhow::Result;
use askama::{Result as AskamaResult, Template, Values};

use super::{FormatterContext, OutputFormatter, ReportData, Verbosity, ViewOptions};
use crate::{
	analysis::AnalysisResults,
	display::report::{DirRecord, EmbeddedRow, LanguageRecord, Summary},
};

/// Escape Markdown table cells by escaping the pipe separator.
#[askama::filter_fn]
pub fn md_escape(value: &str, _values: &dyn Values) -> AskamaResult<String> {
	// A line break would end the table row.
	Ok(value.replace('|', "\\|").replace(['\r', '\n'], " "))
}

mod filters {
	pub use super::md_escape;
	pub use crate::display::template_filters::{fmt_float, fmt_number, fmt_percent};
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
	directories: &'a [DirRecord<'a>],
	/// Whether to show the language or directory breakdown at all (`-q` shows only the summary).
	show_breakdown: bool,
	by_dir: bool,
	embedded: Vec<EmbeddedRow<'a>>,
	show_files: bool,
	hidden_note: Option<String>,
}

pub struct MarkdownFormatter;

impl OutputFormatter for MarkdownFormatter {
	fn write_output(
		&self,
		results: &AnalysisResults,
		paths: &[PathBuf],
		view_options: ViewOptions,
		writer: &mut dyn Write,
	) -> Result<()> {
		let (ctx, report) = self.prepare_report(results, paths, view_options);
		Self::write_markdown(&report, view_options.verbosity, &ctx, writer)
	}
}

impl MarkdownFormatter {
	fn write_markdown(
		report: &ReportData,
		verbosity: Verbosity,
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
			directories: &report.directories,
			show_breakdown: verbosity > Verbosity::Summary,
			by_dir: ctx.options.by_dir,
			embedded: report.embedded_rows(),
			show_files: verbosity == Verbosity::Verbose,
			hidden_note: report.hidden_note(ctx.options.by_dir),
		};
		let rendered = template.render()?;
		// Askama drops the template's final newline.
		writeln!(writer, "{rendered}")?;
		Ok(())
	}
}
