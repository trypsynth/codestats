//! Output formatting and display options for analysis results.
//!
//! ## Available Formatters
//!
//! - CSV ([`CsvFormatter`]): Comma-separated values for spreadsheet import.
//! - Human ([`HumanFormatter`]): Friendly, readable text output for terminal display.
//! - HTML ([`HtmlFormatter`]): Standalone HTML report.
//! - JSON ([`JsonFormatter`]): Pretty-printed JSON for easy processing and reading.
//! - JSON Compact ([`JsonCompactFormatter`]): Minified JSON for minimal bandwidth.
//! - Markdown ([`MarkdownFormatter`]): GitHub-flavored markdown for documentation.
//! - TSV ([`TsvFormatter`]): Tab-separated values for data pipelines.
//!
//! ## Customization Options
//!
//! All formatters support common display options via [`ViewOptions`]:
//! - Number formatting: plain, comma-separated, underscore-separated, or space-separated.
//! - Size units: binary (KiB/MiB) or decimal (KB/MB).
//! - Percentage precision: 0-6 decimal places.
//! - Sort order: by lines, code, comments, blanks, files, size, or filename.
//! - Sort direction: ascending or descending.
//! - Indentation style: tab or 1-8 spaces.

pub mod formatting;
mod html;
mod human;
mod json;
mod markdown;
mod options;
mod report;
mod separated_values;
pub mod template_filters;

use std::{io::Write, path::Path};

use anyhow::Result;
use clap::ValueEnum;
pub use formatting::{FormatterContext, apply_sort};
pub use html::HtmlFormatter;
pub use human::HumanFormatter;
pub use json::{JsonCompactFormatter, JsonFormatter};
pub use markdown::MarkdownFormatter;
pub use options::{IndentStyle, LanguageSortKey, NumberStyle, SizeStyle, SortDirection, Verbosity, ViewOptions};
pub use report::ReportData;
pub use separated_values::{CsvFormatter, TsvFormatter};
use serde::{Deserialize, Serialize};

use crate::analysis::AnalysisResults;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFormat {
	Human,
	Json,
	JsonCompact,
	Csv,
	Tsv,
	Markdown,
	Html,
}

/// Trait for formatting analysis results in a desired format.
pub trait OutputFormatter {
	/// Format and stream the analysis results to the provided writer
	///
	/// # Errors
	///
	/// Returns an error if formatting fails, (e.g. JSON serialization encounters an issue).
	fn write_output(
		&self,
		results: &AnalysisResults,
		path: &Path,
		view_options: ViewOptions,
		writer: &mut dyn Write,
	) -> Result<()>;

	fn prepare_report<'a>(
		&self,
		results: &'a AnalysisResults,
		path: &Path,
		view_options: ViewOptions,
	) -> (FormatterContext, ReportData<'a>) {
		let ctx = FormatterContext::new(view_options);
		let report = ReportData::from_results(results, path, view_options.verbosity, &ctx);
		(ctx, report)
	}
}

/// Format and stream the analysis results to `writer` in the given format.
///
/// # Errors
///
/// Returns an error if formatting or writing fails.
pub fn write_output(
	format: OutputFormat,
	results: &AnalysisResults,
	path: &Path,
	view_options: ViewOptions,
	writer: &mut dyn Write,
) -> Result<()> {
	match format {
		OutputFormat::Human => HumanFormatter.write_output(results, path, view_options, writer),
		OutputFormat::Json => JsonFormatter.write_output(results, path, view_options, writer),
		OutputFormat::JsonCompact => JsonCompactFormatter.write_output(results, path, view_options, writer),
		OutputFormat::Csv => CsvFormatter::default().write_output(results, path, view_options, writer),
		OutputFormat::Tsv => TsvFormatter::default().write_output(results, path, view_options, writer),
		OutputFormat::Markdown => MarkdownFormatter.write_output(results, path, view_options, writer),
		OutputFormat::Html => HtmlFormatter.write_output(results, path, view_options, writer),
	}
}
