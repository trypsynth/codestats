use std::{io::Write, path::Path};

use anyhow::Result;
use serde_json::to_writer_pretty;

use super::{OutputFormatter, ReportData};
use crate::analysis::AnalysisResults;

pub struct JsonFormatter;

impl OutputFormatter for JsonFormatter {
	fn write_output(
		&self,
		results: &AnalysisResults,
		path: &Path,
		verbose: bool,
		writer: &mut dyn Write,
	) -> Result<()> {
		let report = ReportData::from_results(results, path, verbose);
		to_writer_pretty(writer, &report)?;
		Ok(())
	}
}
