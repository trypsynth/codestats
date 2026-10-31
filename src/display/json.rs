use std::{io::Write, path::Path};

use anyhow::Result;
use serde::Serialize;
use serde_json::ser::{PrettyFormatter, Serializer};

use super::{OutputFormatter, ReportData, ViewOptions};
use crate::{
	analysis::{AnalysisResults, LineCounts},
	display::{options::IndentStyle, report::FileRecord},
};

/// Version of the JSON output's shape, raised whenever a field is renamed, removed or changes meaning.
const SCHEMA_VERSION: u32 = 1;

#[derive(Serialize)]
struct Report<'a> {
	schema_version: u32,
	path: &'a str,
	totals: Totals,
	languages: Vec<Group<'a>>,
	hidden_languages: usize,
	#[serde(skip_serializing_if = "Option::is_none")]
	directories: Option<Vec<Group<'a>>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	hidden_directories: Option<usize>,
}

#[derive(Serialize)]
struct Totals {
	file_count: u64,
	lines: LineCounts,
	bytes: u64,
}

/// A language, or a directory with `--by-dir`.
#[derive(Serialize)]
struct Group<'a> {
	name: &'a str,
	file_count: u64,
	lines: LineCounts,
	bytes: u64,
	#[serde(skip_serializing_if = "Option::is_none")]
	files: Option<Vec<File<'a>>>,
}

#[derive(Serialize)]
struct File<'a> {
	path: &'a str,
	lines: LineCounts,
	bytes: u64,
}

impl<'a> Report<'a> {
	fn new(report: &'a ReportData, by_dir: bool) -> Self {
		let files = |records: &'a Option<Vec<FileRecord>>| {
			records.as_ref().map(|records| {
				records.iter().map(|file| File { path: file.path, lines: file.counts, bytes: file.size }).collect()
			})
		};
		let summary = &report.summary;
		Self {
			schema_version: SCHEMA_VERSION,
			path: &report.analysis_path,
			totals: Totals { file_count: summary.total_files, lines: summary.counts, bytes: summary.total_size },
			languages: report
				.languages
				.iter()
				.map(|lang| Group {
					name: lang.name,
					file_count: lang.files,
					lines: lang.counts,
					bytes: lang.size,
					files: files(&lang.files_detail),
				})
				.collect(),
			hidden_languages: report.languages_hidden.unwrap_or(0),
			directories: by_dir.then(|| {
				report
					.directories
					.iter()
					.map(|dir| Group {
						name: &dir.path,
						file_count: dir.files,
						lines: dir.counts,
						bytes: dir.size,
						files: files(&dir.files_detail),
					})
					.collect()
			}),
			hidden_directories: by_dir.then(|| report.dirs_hidden.unwrap_or(0)),
		}
	}
}

fn write_json(
	formatter: &impl OutputFormatter,
	results: &AnalysisResults,
	path: &Path,
	view_options: ViewOptions,
	writer: &mut dyn Write,
	pretty: bool,
) -> Result<()> {
	let (_ctx, report) = formatter.prepare_report(results, path, view_options);
	write_value(&Report::new(&report, view_options.by_dir), view_options, writer, pretty)
}

/// Write `value` as JSON, pretty-printed with the configured indent or on one line, ending with a newline.
pub(super) fn write_value(
	value: &impl Serialize,
	view_options: ViewOptions,
	writer: &mut dyn Write,
	pretty: bool,
) -> Result<()> {
	if pretty {
		let indent_bytes: Vec<u8> = match view_options.indent_style {
			IndentStyle::Tab => b"\t".to_vec(),
			IndentStyle::Spaces(n) => vec![b' '; usize::from(n)],
		};
		let formatter = PrettyFormatter::with_indent(&indent_bytes);
		let mut ser = Serializer::with_formatter(&mut *writer, formatter);
		value.serialize(&mut ser)?;
	} else {
		serde_json::to_writer(&mut *writer, value)?;
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
		view_options: ViewOptions,
		writer: &mut dyn Write,
	) -> Result<()> {
		write_json(self, results, path, view_options, writer, true)
	}
}

pub struct JsonCompactFormatter;

impl OutputFormatter for JsonCompactFormatter {
	fn write_output(
		&self,
		results: &AnalysisResults,
		path: &Path,
		view_options: ViewOptions,
		writer: &mut dyn Write,
	) -> Result<()> {
		write_json(self, results, path, view_options, writer, false)
	}
}

#[cfg(test)]
mod tests {
	use std::path::Path;

	use super::*;
	use crate::{
		analysis::AnalysisResults,
		display::{ViewOptions, options::IndentStyle},
	};

	fn render(by_dir: bool, verbosity: crate::display::Verbosity) -> serde_json::Value {
		let mut results = AnalysisResults::default();
		let rust = crate::langs::LANGUAGES.iter().find(|l| l.name == "Rust").unwrap();
		let counts = LineCounts { total: 10, code: 7, comment: 2, blank: 1, shebang: 0 };
		results.add_file(rust, &counts, 100, Some("src/main.rs".to_owned()));
		let options = ViewOptions { verbosity, by_dir, ..ViewOptions::default() };
		let mut buf = Vec::new();
		JsonCompactFormatter.write_output(&results, Path::new("."), options, &mut buf).unwrap();
		serde_json::from_slice(&buf).unwrap()
	}

	/// An object's keys, sorted (`serde_json::Value` doesn't keep their order).
	fn keys(value: &serde_json::Value) -> Vec<&str> {
		value.as_object().unwrap().keys().map(String::as_str).collect()
	}

	#[test]
	fn schema_has_one_shape_for_counts_at_every_level() {
		use crate::display::Verbosity;
		let report = render(false, Verbosity::Verbose);
		assert_eq!(keys(&report), ["hidden_languages", "languages", "path", "schema_version", "totals"]);
		assert_eq!(report["schema_version"], SCHEMA_VERSION);
		let lines = serde_json::json!({ "total": 10, "code": 7, "comment": 2, "blank": 1, "shebang": 0 });
		assert_eq!(report["totals"], serde_json::json!({ "file_count": 1, "lines": lines, "bytes": 100 }));
		let rust = &report["languages"][0];
		assert_eq!(keys(rust), ["bytes", "file_count", "files", "lines", "name"]);
		assert_eq!(rust["lines"], lines);
		assert_eq!(rust["files"][0], serde_json::json!({ "path": "src/main.rs", "lines": lines, "bytes": 100 }));
	}

	#[test]
	fn directories_only_with_by_dir_and_files_only_when_verbose() {
		use crate::display::Verbosity;
		let regular = render(false, Verbosity::Regular);
		assert!(regular.get("directories").is_none() && regular.get("hidden_directories").is_none());
		assert!(regular["languages"][0].get("files").is_none());
		let by_dir = render(true, Verbosity::Verbose);
		assert_eq!(by_dir["hidden_directories"], 0);
		assert_eq!(by_dir["directories"][0]["name"], "src");
		assert_eq!(by_dir["directories"][0]["files"][0]["path"], "src/main.rs");
	}

	#[test]
	fn no_presentation_fields_leak_into_json() {
		let text = render(true, crate::display::Verbosity::Verbose).to_string();
		for old in ["_human", "percentage", "avg_", "files_detail", "_lines", "analysis_path", "summary"] {
			assert!(!text.contains(old), "{old:?} in {text}");
		}
	}

	#[test]
	fn json_pretty_uses_configured_indent() {
		let results = AnalysisResults::default();
		let options = ViewOptions { indent_style: IndentStyle::Spaces(4), ..Default::default() };
		let formatter = JsonFormatter;
		let mut buf = Vec::new();
		formatter.write_output(&results, Path::new("."), options, &mut buf).unwrap();
		let output = String::from_utf8(buf).unwrap();
		assert!(output.contains("    \""), "expected 4-space indent in JSON, got:\n{output}");
	}

	#[test]
	fn json_pretty_uses_tab_indent() {
		let results = AnalysisResults::default();
		let options = ViewOptions::default(); // default is Tab
		let formatter = JsonFormatter;
		let mut buf = Vec::new();
		formatter.write_output(&results, Path::new("."), options, &mut buf).unwrap();
		let output = String::from_utf8(buf).unwrap();
		assert!(output.contains("\t\""), "expected tab indent in JSON, got:\n{output}");
	}

	#[test]
	fn json_compact_ignores_indent() {
		let results = AnalysisResults::default();
		let options = ViewOptions { indent_style: IndentStyle::Spaces(4), ..Default::default() };
		let formatter = JsonCompactFormatter;
		let mut buf = Vec::new();
		formatter.write_output(&results, Path::new("."), options, &mut buf).unwrap();
		let output = String::from_utf8(buf).unwrap();
		assert!(
			!output.contains('\n') || output.ends_with('\n') && output.matches('\n').count() == 1,
			"compact JSON should be single line"
		);
	}
}
