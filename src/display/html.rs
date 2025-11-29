use std::{io::Write, path::Path};

use anyhow::Result;
use minijinja::{AutoEscape, Environment, context};

use super::{FormatterContext, OutputFormatter, ReportData, ViewOptions};
use crate::{analysis::AnalysisResults, display::report::FormattedLanguage};

const HTML_TEMPLATE: &str = include_str!("templates/report.html");

pub struct HtmlFormatter;

impl OutputFormatter for HtmlFormatter {
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
		let languages = report.formatted_languages(&ctx);
		Self::write_document(&report, &languages, verbose, &ctx, writer)
	}
}

impl HtmlFormatter {
	fn write_document(
		report: &ReportData,
		languages: &[FormattedLanguage],
		verbose: bool,
		ctx: &FormatterContext,
		writer: &mut dyn Write,
	) -> Result<()> {
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
		let parts = report.summary.percentage_parts(ctx);
		let totals = (!parts.is_empty()).then(|| parts.join(", "));
		let rendered = template.render(context! {
			title => &report.analysis_path,
			summary => &report.summary,
			totals,
			languages => languages,
			show_files => verbose,
		})?;
		writer.write_all(rendered.as_bytes())?;
		Ok(())
	}
}
