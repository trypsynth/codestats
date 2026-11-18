//! The `--show-skipped` report: the files that codestats leaves out as generated, vendored, or documentation, by reason, with the lines each would add.

use std::io::Write;

use anyhow::{Result, bail};
use serde::Serialize;

use super::{FormatterContext, OutputFormat, Verbosity, ViewOptions, formatting::pluralize, json::write_value};
use crate::analysis::{AnalysisResults, SkipReason, SkippedFile};

/// The files left out for one reason, sorted by path.
struct Group<'a> {
	reason: SkipReason,
	files: Vec<&'a SkippedFile>,
	lines: u64,
	/// How many of the files are generated too, so that counting them takes `--include-generated` as well.
	also_generated: u64,
}

/// The skipped files grouped by reason, in the order [`SkipReason`] lists them.
fn group(results: &AnalysisResults) -> Vec<Group<'_>> {
	let mut files: Vec<&SkippedFile> = results.skipped_files().iter().collect();
	files.sort_by(|a, b| a.reason.cmp(&b.reason).then_with(|| a.path.cmp(&b.path)));
	let mut groups: Vec<Group> = Vec::new();
	for file in files {
		match groups.last_mut() {
			Some(group) if group.reason == file.reason => {
				group.lines += file.lines;
				group.also_generated += u64::from(file.also_generated);
				group.files.push(file);
			}
			_ => groups.push(Group {
				reason: file.reason,
				files: vec![file],
				lines: file.lines,
				also_generated: u64::from(file.also_generated),
			}),
		}
	}
	groups
}

/// Write the `--show-skipped` report in `format`, which has to be human-readable text or JSON.
///
/// # Errors
///
/// Returns an error for another format, or if writing fails.
pub fn write_skipped(
	format: OutputFormat,
	results: &AnalysisResults,
	view_options: ViewOptions,
	writer: &mut dyn Write,
) -> Result<()> {
	let groups = group(results);
	match format {
		OutputFormat::Human => write_human(&groups, view_options, writer),
		OutputFormat::Json | OutputFormat::JsonCompact => {
			let report = Report::new(&groups, view_options.verbosity == Verbosity::Verbose);
			write_value(&report, view_options, writer, format == OutputFormat::Json)
		}
		_ => bail!("--show-skipped writes human-readable text or JSON, so pass `-o human` or `-o json`"),
	}
}

fn write_human(groups: &[Group], view_options: ViewOptions, writer: &mut dyn Write) -> Result<()> {
	let ctx = FormatterContext::new(view_options);
	let files: u64 = groups.iter().map(|group| group.files.len() as u64).sum();
	if files == 0 {
		writeln!(writer, "No files were skipped.")?;
		return Ok(());
	}
	let lines: u64 = groups.iter().map(|group| group.lines).sum();
	let count = |value: u64, singular, plural| format!("{} {}", ctx.number(value), pluralize(value, singular, plural));
	writeln!(writer, "Skipped {} with {}.", count(files, "file", "files"), count(lines, "line", "lines"))?;
	for group in groups {
		let flags = group.reason.counted_with().join(" or ");
		let also = match group.also_generated {
			0 => String::new(),
			n => format!(
				", and --include-generated too for {} of them, which {} also generated",
				ctx.number(n),
				pluralize(n, "is", "are")
			),
		};
		writeln!(
			writer,
			"\n{}: {}, {}. Pass {flags} to count them{also}.",
			group.reason.describe(),
			count(group.files.len() as u64, "file", "files"),
			count(group.lines, "line", "lines"),
		)?;
		if view_options.verbosity == Verbosity::Verbose {
			let indent = ctx.indent(1);
			for file in &group.files {
				let also = if file.also_generated { ", also generated" } else { "" };
				writeln!(
					writer,
					"{indent}{}: {}, {}{also}.",
					file.path,
					file.language.name,
					count(file.lines, "line", "lines")
				)?;
			}
		}
	}
	Ok(())
}

#[derive(Serialize)]
struct Report<'a> {
	files: u64,
	lines: u64,
	reasons: Vec<ReasonRecord<'a>>,
}

#[derive(Serialize)]
struct ReasonRecord<'a> {
	reason: SkipReason,
	description: &'static str,
	counted_with: &'static [&'static str],
	files: u64,
	lines: u64,
	/// How many of the files are generated too, when any are.
	#[serde(skip_serializing_if = "is_zero")]
	also_generated: u64,
	/// Each file, with `-v`.
	#[serde(skip_serializing_if = "Option::is_none")]
	skipped: Option<Vec<FileRecord<'a>>>,
}

#[derive(Serialize)]
struct FileRecord<'a> {
	path: &'a str,
	language: &'static str,
	lines: u64,
	#[serde(skip_serializing_if = "std::ops::Not::not")]
	also_generated: bool,
}

#[expect(clippy::trivially_copy_pass_by_ref, reason = "serde passes a reference")]
const fn is_zero(value: &u64) -> bool {
	*value == 0
}

impl<'a> Report<'a> {
	fn new(groups: &[Group<'a>], verbose: bool) -> Self {
		let reasons: Vec<ReasonRecord> = groups
			.iter()
			.map(|group| ReasonRecord {
				reason: group.reason,
				description: group.reason.describe(),
				counted_with: group.reason.counted_with(),
				files: group.files.len() as u64,
				lines: group.lines,
				also_generated: group.also_generated,
				skipped: verbose.then(|| {
					group
						.files
						.iter()
						.map(|file| FileRecord {
							path: &file.path,
							language: file.language.name,
							lines: file.lines,
							also_generated: file.also_generated,
						})
						.collect()
				}),
			})
			.collect();
		Self {
			files: reasons.iter().map(|reason| reason.files).sum(),
			lines: reasons.iter().map(|reason| reason.lines).sum(),
			reasons,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::langs::language_named;

	fn results() -> AnalysisResults {
		let go = language_named("Go").unwrap();
		let mut results = AnalysisResults::default();
		let mut skip = |path: &str, reason, also_generated, lines| {
			results.add_skipped(SkippedFile { path: path.to_owned(), reason, also_generated, language: go, lines });
		};
		skip("vendor/b.go", SkipReason::VendoredDirectory, true, 20);
		skip("go.sum", SkipReason::GeneratedName, false, 1);
		skip("vendor/a.go", SkipReason::VendoredDirectory, false, 10);
		results
	}

	fn render(format: OutputFormat, verbosity: Verbosity) -> String {
		let mut buf = Vec::new();
		write_skipped(format, &results(), ViewOptions { verbosity, ..ViewOptions::default() }, &mut buf).unwrap();
		String::from_utf8(buf).unwrap()
	}

	#[test]
	fn human_report_groups_by_reason_and_lists_files_when_verbose() {
		let regular = render(OutputFormat::Human, Verbosity::Regular);
		assert_eq!(
			regular,
			"Skipped 3 files with 31 lines.\n\nGenerated, by name: 1 file, 1 line. Pass --include-generated to count \
			 them.\n\nVendored, in a vendored directory: 2 files, 30 lines. Pass --include-vendored to count them, and \
			 --include-generated too for 1 of them, which is also generated.\n"
		);
		let verbose = render(OutputFormat::Human, Verbosity::Verbose);
		assert!(
			verbose
				.contains("generated.\n\tvendor/a.go: Go, 10 lines.\n\tvendor/b.go: Go, 20 lines, also generated.\n"),
			"{verbose}"
		);
	}

	#[test]
	fn json_report_lists_files_only_when_verbose() {
		let regular: serde_json::Value = serde_json::from_str(&render(OutputFormat::Json, Verbosity::Regular)).unwrap();
		assert_eq!(regular["files"], 3);
		assert_eq!(regular["reasons"][1]["reason"], "vendored-directory");
		assert_eq!(regular["reasons"][1]["counted_with"][0], "--include-vendored");
		assert!(regular["reasons"][1].get("skipped").is_none());
		assert_eq!(regular["reasons"][1]["also_generated"], 1);
		assert!(regular["reasons"][0].get("also_generated").is_none());
		let verbose: serde_json::Value =
			serde_json::from_str(&render(OutputFormat::JsonCompact, Verbosity::Verbose)).unwrap();
		assert_eq!(verbose["reasons"][1]["skipped"][0]["path"], "vendor/a.go");
	}

	#[test]
	fn nothing_skipped_says_so_and_other_formats_are_refused() {
		let mut buf = Vec::new();
		write_skipped(OutputFormat::Human, &AnalysisResults::default(), ViewOptions::default(), &mut buf).unwrap();
		assert_eq!(String::from_utf8(buf).unwrap(), "No files were skipped.\n");
		let refused = write_skipped(OutputFormat::Csv, &results(), ViewOptions::default(), &mut Vec::new());
		assert!(refused.unwrap_err().to_string().contains("-o json"));
	}
}
