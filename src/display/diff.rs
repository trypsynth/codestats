//! Output for `cs diff`: each language's counts in two versions, side by side, in every format.

use std::{cmp::Reverse, fmt::Write as _, io::Write, path::Path};

use anyhow::Result;
use serde::Serialize;

use super::{FormatterContext, OutputFormat, ViewOptions, formatting::pluralize};
use crate::{
	analysis::{AnalysisResults, LineCounts, LineType},
	langs::LANGUAGES,
};

/// Version of the `cs diff` JSON shape, raised whenever a field is renamed, removed or changes meaning.
const SCHEMA_VERSION: u32 = 1;

/// One version's counts for a language, or for everything.
#[derive(Clone, Copy, Default, Serialize)]
struct Side {
	file_count: u64,
	lines: LineCounts,
	bytes: u64,
}

struct Row {
	name: &'static str,
	/// `None` when the language isn't in that version.
	from: Option<Side>,
	to: Option<Side>,
}

impl Row {
	fn line_change(&self) -> i128 {
		i128::from(self.to.unwrap_or_default().lines.total) - i128::from(self.from.unwrap_or_default().lines.total)
	}

	const fn status(&self) -> &'static str {
		match (self.from, self.to) {
			(None, Some(_)) => " (new)",
			(Some(_), None) => " (removed)",
			_ => "",
		}
	}
}

/// Two versions being compared: their names (`None` for the working tree) and counts.
pub struct Versions<'a> {
	pub from_name: &'a str,
	pub to_name: Option<&'a str>,
	pub from: &'a AnalysisResults,
	pub to: &'a AnalysisResults,
}

struct Diff<'a> {
	path: String,
	from_name: &'a str,
	to_name: Option<&'a str>,
	totals: (Side, Side),
	rows: Vec<Row>,
	hidden: usize,
}

impl<'a> Diff<'a> {
	fn new(versions: &Versions<'a>, path: &Path, options: ViewOptions) -> Self {
		let side = |results: &AnalysisResults, index| {
			results.languages().find(|(lang, _)| lang.index == index).map(|(_, stats)| Side {
				file_count: stats.files(),
				lines: *stats.counts(),
				bytes: stats.size(),
			})
		};
		let mut rows: Vec<Row> = LANGUAGES
			.iter()
			.map(|lang| Row {
				name: lang.name,
				from: side(versions.from, lang.index),
				to: side(versions.to, lang.index),
			})
			.filter(|row| row.from.is_some() || row.to.is_some())
			.collect();
		// Biggest changes first; languages that didn't change at all come last, largest first.
		rows.sort_by_key(|row| {
			(
				Reverse(row.line_change().unsigned_abs()),
				Reverse(row.to.or(row.from).unwrap_or_default().lines.total),
				row.name,
			)
		});
		let total = rows.len();
		if let Some(n) = options.top_languages {
			rows.truncate(n);
		}
		let totals = |results: &AnalysisResults| Side {
			file_count: results.total_files(),
			lines: *results.counts(),
			bytes: results.total_size(),
		};
		Self {
			path: path.display().to_string(),
			from_name: versions.from_name,
			to_name: versions.to_name,
			totals: (totals(versions.from), totals(versions.to)),
			hidden: total - rows.len(),
			rows,
		}
	}

	fn to_description(&self) -> String {
		self.to_name.map_or_else(|| "the working tree".to_owned(), |name| format!("`{name}`"))
	}
}

/// A change between two counts, signed: `+21`, `-3`, or `0`.
fn change(ctx: &FormatterContext, from: u64, to: u64) -> String {
	match to.cmp(&from) {
		std::cmp::Ordering::Greater => format!("+{}", ctx.number(to - from)),
		std::cmp::Ordering::Less => format!("-{}", ctx.number(from - to)),
		std::cmp::Ordering::Equal => "0".to_owned(),
	}
}

/// "from → to (change)" for a table cell.
fn cell(ctx: &FormatterContext, from: u64, to: u64) -> String {
	if from == to {
		ctx.number(to)
	} else {
		format!("{} → {} ({})", ctx.number(from), ctx.number(to), change(ctx, from, to))
	}
}

fn size_cell(ctx: &FormatterContext, from: u64, to: u64) -> String {
	if from == to {
		return ctx.size(to);
	}
	let sign = if to > from { '+' } else { '-' };
	format!("{} → {} ({sign}{})", ctx.size(from), ctx.size(to), ctx.size(from.abs_diff(to)))
}

const LINE_TYPES: [LineType; 4] = [LineType::Code, LineType::Comment, LineType::Blank, LineType::Shebang];

/// The line types that changed, like "+900 code, +80 comments".
fn line_type_changes(ctx: &FormatterContext, from: &LineCounts, to: &LineCounts) -> Vec<String> {
	LINE_TYPES
		.iter()
		.filter(|&&kind| from.get(kind) != to.get(kind))
		.map(|&kind| {
			let (from, to) = (from.get(kind), to.get(kind));
			format!(
				"{} {}",
				change(ctx, from, to),
				pluralize(from.abs_diff(to), kind.singular_label(), kind.plural_label())
			)
		})
		.collect()
}

/// Write a comparison of two versions' counts.
///
/// # Errors
///
/// Returns an error if writing fails.
pub fn write_diff(
	format: OutputFormat,
	versions: &Versions,
	path: &Path,
	options: ViewOptions,
	writer: &mut dyn Write,
) -> Result<()> {
	let ctx = FormatterContext::new(options);
	let diff = Diff::new(versions, path, options);
	match format {
		OutputFormat::Human => write_human(&diff, &ctx, writer),
		OutputFormat::Json => write_json(&diff, options, writer, true),
		OutputFormat::JsonCompact => write_json(&diff, options, writer, false),
		OutputFormat::Csv => write_separated(&diff, b',', writer),
		OutputFormat::Tsv => write_separated(&diff, b'\t', writer),
		OutputFormat::Markdown => write_markdown(&diff, &ctx, writer),
		OutputFormat::Html => write_html(&diff, &ctx, writer),
	}
}

fn write_human(diff: &Diff, ctx: &FormatterContext, writer: &mut dyn Write) -> Result<()> {
	let (from, to) = diff.totals;
	writeln!(writer, "Codestats diff for {} from `{}` to {}:", diff.path, diff.from_name, diff.to_description())?;
	let mut lines = format!(
		"Files: {}. Lines: {}",
		cell(ctx, from.file_count, to.file_count),
		cell(ctx, from.lines.total, to.lines.total)
	);
	let types = line_type_changes(ctx, &from.lines, &to.lines);
	if !types.is_empty() {
		write!(lines, ": {}", types.join(", "))?;
	}
	writeln!(writer, "{lines}. Size: {}.", size_cell(ctx, from.bytes, to.bytes))?;
	if diff.rows.is_empty() {
		writeln!(writer, "No recognized programming languages found.")?;
		return Ok(());
	}
	writeln!(writer, "Languages:")?;
	let indent = ctx.indent(1);
	for row in &diff.rows {
		let (from, to) = (row.from.unwrap_or_default(), row.to.unwrap_or_default());
		let mut line = format!(
			"{indent}{}{}: {} {}, {} {}",
			row.name,
			row.status(),
			cell(ctx, from.lines.total, to.lines.total),
			pluralize(to.lines.total.max(from.lines.total), "line", "lines"),
			cell(ctx, from.file_count, to.file_count),
			pluralize(to.file_count.max(from.file_count), "file", "files"),
		);
		let types = line_type_changes(ctx, &from.lines, &to.lines);
		if !types.is_empty() {
			write!(line, "; {}", types.join(", "))?;
		}
		writeln!(writer, "{line}.")?;
	}
	if diff.hidden > 0 {
		writeln!(
			writer,
			"({} more {} not shown)",
			diff.hidden,
			pluralize(diff.hidden as u64, "language", "languages")
		)?;
	}
	Ok(())
}

#[derive(Serialize)]
struct JsonDiff<'a> {
	schema_version: u32,
	path: &'a str,
	from: &'a str,
	/// `None` for the working tree.
	to: Option<&'a str>,
	totals: JsonPair,
	languages: Vec<JsonLanguage>,
	hidden_languages: usize,
}

#[derive(Serialize)]
struct JsonPair {
	from: Side,
	to: Side,
}

#[derive(Serialize)]
struct JsonLanguage {
	name: &'static str,
	/// `None` when the language isn't in that version.
	from: Option<Side>,
	to: Option<Side>,
}

fn write_json(diff: &Diff, options: ViewOptions, writer: &mut dyn Write, pretty: bool) -> Result<()> {
	let json = JsonDiff {
		schema_version: SCHEMA_VERSION,
		path: &diff.path,
		from: diff.from_name,
		to: diff.to_name,
		totals: JsonPair { from: diff.totals.0, to: diff.totals.1 },
		languages: diff.rows.iter().map(|row| JsonLanguage { name: row.name, from: row.from, to: row.to }).collect(),
		hidden_languages: diff.hidden,
	};
	super::json::write_value(&json, options, writer, pretty)
}

fn write_separated(diff: &Diff, delimiter: u8, writer: &mut dyn Write) -> Result<()> {
	let delimiter = char::from(delimiter);
	let mut header = vec!["language".to_owned()];
	for field in ["files", "lines", "code_lines", "comment_lines", "blank_lines", "shebang_lines", "bytes"] {
		header.extend([format!("{field}_from"), format!("{field}_to")]);
	}
	writeln!(writer, "{}", header.join(&delimiter.to_string()))?;
	let mut write_row = |name: &str, from: Side, to: Side| -> Result<()> {
		let values = |side: Side| {
			[
				side.file_count,
				side.lines.total,
				side.lines.code,
				side.lines.comment,
				side.lines.blank,
				side.lines.shebang,
				side.bytes,
			]
		};
		let mut fields = vec![name.to_owned()];
		for (from, to) in values(from).into_iter().zip(values(to)) {
			fields.extend([from.to_string(), to.to_string()]);
		}
		writeln!(writer, "{}", fields.join(&delimiter.to_string()))?;
		Ok(())
	};
	write_row("(total)", diff.totals.0, diff.totals.1)?;
	for row in &diff.rows {
		write_row(row.name, row.from.unwrap_or_default(), row.to.unwrap_or_default())?;
	}
	Ok(())
}

/// The table both Markdown and HTML show: a header and one row of cells per language, starting with the totals.
fn table(diff: &Diff, ctx: &FormatterContext) -> (Vec<&'static str>, Vec<Vec<String>>) {
	let header = vec!["Language", "Files", "Lines", "Code", "Comments", "Blanks", "Size"];
	let row = |name: String, from: Side, to: Side| {
		let mut cells =
			vec![name, cell(ctx, from.file_count, to.file_count), cell(ctx, from.lines.total, to.lines.total)];
		cells.extend(LINE_TYPES[..3].iter().map(|&kind| cell(ctx, from.lines.get(kind), to.lines.get(kind))));
		cells.push(size_cell(ctx, from.bytes, to.bytes));
		cells
	};
	let mut rows = vec![row("Total".to_owned(), diff.totals.0, diff.totals.1)];
	rows.extend(
		diff.rows
			.iter()
			.map(|r| row(format!("{}{}", r.name, r.status()), r.from.unwrap_or_default(), r.to.unwrap_or_default())),
	);
	(header, rows)
}

fn write_markdown(diff: &Diff, ctx: &FormatterContext, writer: &mut dyn Write) -> Result<()> {
	let escape = |text: &str| text.replace('|', "\\|");
	writeln!(writer, "# Codestats diff for `{}`\n", diff.path)?;
	writeln!(writer, "From `{}` to {}.\n", diff.from_name, diff.to_description())?;
	let (header, rows) = table(diff, ctx);
	writeln!(writer, "| {} |", header.join(" | "))?;
	writeln!(writer, "| --- |{}", " ---: |".repeat(header.len() - 1))?;
	for row in rows {
		writeln!(writer, "| {} |", row.iter().map(|cell| escape(cell)).collect::<Vec<_>>().join(" | "))?;
	}
	if diff.hidden > 0 {
		writeln!(
			writer,
			"\n_{} more {} not shown._",
			diff.hidden,
			pluralize(diff.hidden as u64, "language", "languages")
		)?;
	}
	Ok(())
}

fn write_html(diff: &Diff, ctx: &FormatterContext, writer: &mut dyn Write) -> Result<()> {
	let escape =
		|text: &str| text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;");
	let title = format!("Codestats diff for {}", escape(&diff.path));
	let to =
		diff.to_name.map_or_else(|| "the working tree".to_owned(), |name| format!("<code>{}</code>", escape(name)));
	writeln!(writer, "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n\t<meta charset=\"utf-8\" />")?;
	writeln!(
		writer,
		"\t<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n\t<title>{title}</title>\n</head>"
	)?;
	writeln!(
		writer,
		"<body>\n\t<h1>{title}</h1>\n\t<p>From <code>{}</code> to {to}.</p>\n\t<table>",
		escape(diff.from_name)
	)?;
	let (header, rows) = table(diff, ctx);
	write!(writer, "\t\t<tr>")?;
	for heading in &header {
		write!(writer, "<th>{heading}</th>")?;
	}
	writeln!(writer, "</tr>")?;
	for row in rows {
		write!(writer, "\t\t<tr>")?;
		for cell in &row {
			write!(writer, "<td>{}</td>", escape(cell))?;
		}
		writeln!(writer, "</tr>")?;
	}
	writeln!(writer, "\t</table>")?;
	if diff.hidden > 0 {
		writeln!(
			writer,
			"\t<p>{} more {} not shown.</p>",
			diff.hidden,
			pluralize(diff.hidden as u64, "language", "languages")
		)?;
	}
	writeln!(writer, "</body>\n</html>")?;
	Ok(())
}
