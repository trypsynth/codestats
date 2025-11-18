mod csv;
mod human;
mod json;

use std::{
	fmt::{self, Display},
	path::Path,
};

use anyhow::Result;
use clap::ValueEnum;
pub use csv::CsvFormatter;
pub use human::HumanFormatter;
pub use json::JsonFormatter;

use crate::analysis::AnalysisResults;

/// Available output formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
	Human,
	Json,
	Csv,
}

impl Display for OutputFormat {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Human => write!(f, "human"),
			Self::Json => write!(f, "json"),
			Self::Csv => write!(f, "csv"),
		}
	}
}

/// Trait for formatting analysis results in a desired format.
pub trait OutputFormatter {
	/// Format and output the analysis results
	///
	/// # Errors
	///
	/// Returns an error if formatting fails, (e.g. JSON serialization encounters an issue).
	fn format(&self, results: &AnalysisResults, path: &Path, verbose: bool) -> Result<String>;
}

/// Factory for creating output formatters
#[must_use]
pub fn get_formatter(format: OutputFormat) -> Box<dyn OutputFormatter> {
	match format {
		OutputFormat::Human => Box::new(HumanFormatter),
		OutputFormat::Json => Box::new(JsonFormatter),
		OutputFormat::Csv => Box::new(CsvFormatter),
	}
}
