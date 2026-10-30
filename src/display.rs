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

#[cfg(test)]
mod tests {
	use std::path::Path;

	use super::{OutputFormat, Verbosity, ViewOptions, write_output};
	use crate::analysis::{AnalysisResults, LineCounts};

	fn render(format: OutputFormat, verbosity: Verbosity) -> String {
		let mut results = AnalysisResults::default();
		let rust = crate::langs::LANGUAGES.iter().find(|l| l.name == "Rust").unwrap();
		let counts = LineCounts { total: 10, code: 10, ..LineCounts::default() };
		for path in ["src/main.rs", "tests/smoke.rs"] {
			results.add_file(rust, &counts, 100, Some(path.to_owned()));
		}
		let options = ViewOptions { by_dir: true, verbosity, ..ViewOptions::default() };
		let mut buf = Vec::new();
		write_output(format, &results, Path::new("."), options, &mut buf).unwrap();
		String::from_utf8(buf).unwrap()
	}

	#[test]
	fn by_dir_replaces_the_language_breakdown_in_every_format() {
		for format in [OutputFormat::Csv, OutputFormat::Tsv, OutputFormat::Markdown, OutputFormat::Html] {
			for verbosity in [Verbosity::Regular, Verbosity::Verbose] {
				let output = render(format, verbosity);
				let context = format!("{format:?} {verbosity:?}:\n{output}");
				assert!(output.to_lowercase().contains("director"), "no directory breakdown in {context}");
				assert!(output.contains("src") && output.contains("tests"), "directories missing in {context}");
				assert!(
					!output.contains("Average Lines") && !output.contains("avg_lines"),
					"language table in {context}"
				);
				let lists_files = output.contains("src/main.rs");
				assert_eq!(lists_files, verbosity == Verbosity::Verbose, "file listing in {context}");
			}
		}
	}
}
