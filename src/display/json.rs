use std::{io::Write, path::Path};

use anyhow::Result;
use serde_json::{to_writer, to_writer_pretty};

use super::{OutputFormatter, ViewOptions};
use crate::analysis::AnalysisResults;

fn write_json(
	formatter: &impl OutputFormatter,
	results: &AnalysisResults,
	path: &Path,
	verbose: bool,
	view_options: ViewOptions,
	writer: &mut dyn Write,
	pretty: bool,
) -> Result<()> {
	let (_ctx, report) = formatter.prepare_report(results, path, verbose, view_options);
	if pretty {
		to_writer_pretty(&mut *writer, &report)?;
	} else {
		to_writer(&mut *writer, &report)?;
	}
	writeln!(writer)?;
	Ok(())
}

pub struct JsonFormatter;

impl OutputFormatter for JsonFormatter {
	fn write_output(
		&self,
		results: &AnalysisResults,
		path: &Path,
		verbose: bool,
		view_options: ViewOptions,
		writer: &mut dyn Write,
	) -> Result<()> {
		write_json(self, results, path, verbose, view_options, writer, true)
	}
}

pub struct JsonCompactFormatter;

impl OutputFormatter for JsonCompactFormatter {
	fn write_output(
		&self,
		results: &AnalysisResults,
		path: &Path,
		verbose: bool,
		view_options: ViewOptions,
		writer: &mut dyn Write,
	) -> Result<()> {
		write_json(self, results, path, verbose, view_options, writer, false)
	}
}
