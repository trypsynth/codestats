use std::{io::Write, path::Path};

use anyhow::Result;
use serde_json::to_writer_pretty;

use super::{FormatterContext, OutputFormatter, ReportData, ViewOptions};
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
		let ctx = FormatterContext::new(view_options);
		let report = ReportData::from_results(results, path, verbose, &ctx);
		to_writer_pretty(writer, &report)?;
		Ok(())
	}
}
