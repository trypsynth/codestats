use std::{io::Write, path::Path};

use anyhow::Result;
use minijinja::{AutoEscape, Environment, context};
use serde::Serialize;

use super::{OutputFormatter, ReportData};
use crate::analysis::AnalysisResults;

const HTML_TEMPLATE: &str = include_str!("templates/report.html");

#[derive(Serialize)]
struct LanguageView<'a> {
	name: &'a str,
	files: u64,
	lines: u64,
	code_percentage: String,
	comment_percentage: String,
	blank_percentage: String,
	shebang_percentage: String,
	size_human: &'a str,
	files_detail: Option<Vec<FileView<'a>>>,
}

#[derive(Serialize)]
struct FileView<'a> {
	path: &'a str,
	total_lines: u64,
	code_lines: u64,
	comment_lines: u64,
	blank_lines: u64,
	shebang_lines: u64,
	size_human: &'a str,
}

pub struct HtmlFormatter;

impl OutputFormatter for HtmlFormatter {
	fn write_output(
		&self,
		results: &AnalysisResults,
		path: &Path,
		verbose: bool,
		writer: &mut dyn Write,
	) -> Result<()> {
		let report = ReportData::from_results(results, path, verbose);
		Self::write_document(&report, verbose, writer)
	}
}

impl HtmlFormatter {
	fn write_document(report: &ReportData, verbose: bool, writer: &mut dyn Write) -> Result<()> {
		let mut env = Environment::new();
		env.set_auto_escape_callback(|name| {
			if Path::new(name).extension().is_some_and(|ext| ext.eq_ignore_ascii_case("html")) {
				AutoEscape::Html
			} else {
				AutoEscape::None
			}
		});
		env.add_template("report.html", HTML_TEMPLATE)?;
		let template = env.get_template("report.html")?;
		let languages = render_languages(report);
		let parts = report.summary.percentage_parts();
		let totals = (!parts.is_empty()).then(|| parts.join(", "));
		let rendered = template.render(context! {
			title => &report.analysis_path,
			summary => &report.summary,
			totals,
			languages => &languages,
			show_files => verbose,
		})?;
		writer.write_all(rendered.as_bytes())?;
		Ok(())
	}
}

fn render_languages<'a>(report: &'a ReportData<'a>) -> Vec<LanguageView<'a>> {
	report
		.languages
		.iter()
		.map(|lang| LanguageView {
			name: lang.name,
			files: lang.files,
			lines: lang.lines,
			code_percentage: format!("{:.1}", lang.code_percentage),
			comment_percentage: format!("{:.1}", lang.comment_percentage),
			blank_percentage: format!("{:.1}", lang.blank_percentage),
			shebang_percentage: format!("{:.1}", lang.shebang_percentage),
			size_human: &lang.size_human,
			files_detail: lang.files_detail.as_ref().map(|files| {
				files
					.iter()
					.map(|file| FileView {
						path: file.path,
						total_lines: file.total_lines,
						code_lines: file.code_lines,
						comment_lines: file.comment_lines,
						blank_lines: file.blank_lines,
						shebang_lines: file.shebang_lines,
						size_human: &file.size_human,
					})
					.collect()
			}),
		})
		.collect()
}
