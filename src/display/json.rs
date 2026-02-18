use std::{io::Write, path::Path};

use anyhow::Result;
use serde::Serialize;
use serde_json::ser::{PrettyFormatter, Serializer};

use super::{OutputFormatter, ViewOptions};
use crate::{analysis::AnalysisResults, display::options::IndentStyle};

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
		let indent_bytes: Vec<u8> = match view_options.indent_style {
			IndentStyle::Tab => b"\t".to_vec(),
			IndentStyle::Spaces(n) => vec![b' '; usize::from(n)],
		};
		let formatter = PrettyFormatter::with_indent(&indent_bytes);
		let mut ser = Serializer::with_formatter(&mut *writer, formatter);
		report.serialize(&mut ser)?;
	} else {
		serde_json::to_writer(&mut *writer, &report)?;
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

#[cfg(test)]
mod tests {
	use std::path::Path;

	use crate::{
		analysis::AnalysisResults,
		display::{ViewOptions, options::IndentStyle},
	};

	use super::*;

	#[test]
	fn json_pretty_uses_configured_indent() {
		let results = AnalysisResults::default();
		let mut options = ViewOptions::default();
		options.indent_style = IndentStyle::Spaces(4);
		let formatter = JsonFormatter;
		let mut buf = Vec::new();
		formatter.write_output(&results, Path::new("."), false, options, &mut buf).unwrap();
		let output = String::from_utf8(buf).unwrap();
		assert!(output.contains("    \""), "expected 4-space indent in JSON, got:\n{output}");
	}

	#[test]
	fn json_pretty_uses_tab_indent() {
		let results = AnalysisResults::default();
		let options = ViewOptions::default(); // default is Tab
		let formatter = JsonFormatter;
		let mut buf = Vec::new();
		formatter.write_output(&results, Path::new("."), false, options, &mut buf).unwrap();
		let output = String::from_utf8(buf).unwrap();
		assert!(output.contains("\t\""), "expected tab indent in JSON, got:\n{output}");
	}

	#[test]
	fn json_compact_ignores_indent() {
		let results = AnalysisResults::default();
		let mut options = ViewOptions::default();
		options.indent_style = IndentStyle::Spaces(4);
		let formatter = JsonCompactFormatter;
		let mut buf = Vec::new();
		formatter.write_output(&results, Path::new("."), false, options, &mut buf).unwrap();
		let output = String::from_utf8(buf).unwrap();
		assert!(!output.contains('\n') || output.ends_with('\n') && output.matches('\n').count() == 1,
			"compact JSON should be single line");
	}
}
