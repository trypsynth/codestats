use std::{io::Write, path::Path};

use anyhow::Result;
use serde_json::{to_writer, to_writer_pretty};

use super::{OutputFormatter, ViewOptions};
use crate::analysis::AnalysisResults;

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
		let (_ctx, report) = self.prepare_report(results, path, verbose, view_options);
		to_writer_pretty(&mut *writer, &report)?;
		writeln!(writer)?;
		Ok(())
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
		let (_ctx, report) = self.prepare_report(results, path, verbose, view_options);
		to_writer(&mut *writer, &report)?;
		writeln!(writer)?;
		Ok(())
	}
}
