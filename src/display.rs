mod formatting;
mod html;
mod human;
mod json;
mod json_compact;
mod markdown;
mod options;
mod report;
mod separated_values;

use std::{
	fmt::{self, Display},
	io::Write,
	path::Path,
};

use anyhow::Result;
use clap::ValueEnum;
pub use formatting::{FormatterContext, apply_sort};
pub use html::HtmlFormatter;
pub use human::HumanFormatter;
pub use json::JsonFormatter;
pub use json_compact::JsonCompactFormatter;
pub use markdown::MarkdownFormatter;
pub use options::{LanguageSortKey, NumberStyle, SizeStyle, SortDirection, ViewOptions};
pub use report::ReportData;
use serde::{Deserialize, Serialize};
pub use separated_values::{CsvFormatter, TsvFormatter};

use crate::analysis::AnalysisResults;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Deserialize, Serialize)]
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

impl Display for OutputFormat {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Human => write!(f, "human"),
			Self::Json => write!(f, "json"),
			Self::JsonCompact => write!(f, "json-compact"),
			Self::Csv => write!(f, "csv"),
			Self::Tsv => write!(f, "tsv"),
			Self::Markdown => write!(f, "markdown"),
			Self::Html => write!(f, "html"),
		}
	}
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
		verbose: bool,
		view_options: ViewOptions,
		writer: &mut dyn Write,
	) -> Result<()>;

	fn prepare_report<'a>(
		&self,
		results: &'a AnalysisResults,
		path: &Path,
		verbose: bool,
		view_options: ViewOptions,
	) -> (FormatterContext, ReportData<'a>) {
		let ctx = FormatterContext::new(view_options);
		let report = ReportData::from_results(results, path, verbose, &ctx);
		(ctx, report)
	}
}

#[must_use]
pub fn get_formatter(format: OutputFormat) -> Box<dyn OutputFormatter> {
	match format {
		OutputFormat::Human => Box::new(HumanFormatter),
		OutputFormat::Json => Box::new(JsonFormatter),
		OutputFormat::JsonCompact => Box::new(JsonCompactFormatter),
		OutputFormat::Csv => Box::new(CsvFormatter::default()),
		OutputFormat::Tsv => Box::new(TsvFormatter::default()),
		OutputFormat::Markdown => Box::new(MarkdownFormatter),
		OutputFormat::Html => Box::new(HtmlFormatter),
	}
}
