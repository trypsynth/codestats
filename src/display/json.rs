use std::{collections::HashMap, path::Path};

use anyhow::Result;
use serde_json::json;

use super::OutputFormatter;
use crate::analysis::AnalysisResults;

/// JSON formatter
pub struct JsonFormatter;

impl OutputFormatter for JsonFormatter {
	fn format(&self, results: &AnalysisResults, path: &Path, verbose: bool) -> Result<String> {
		if verbose {
			let languages: HashMap<_, _> = results
				.languages_by_lines()
				.iter()
				.map(|(lang_name, lang_stats)| {
					(
						lang_name.as_str(),
						json!({
							"files": lang_stats.files(),
							"lines": lang_stats.lines(),
							"code_lines": lang_stats.code_lines(),
							"comment_lines": lang_stats.comment_lines(),
							"blank_lines": lang_stats.blank_lines(),
							"shebang_lines": lang_stats.shebang_lines(),
							"size": lang_stats.size(),
							"size_human": lang_stats.size_human(),
							"code_percentage": lang_stats.code_percentage(),
							"comment_percentage": lang_stats.comment_percentage(),
							"blank_percentage": lang_stats.blank_percentage(),
							"shebang_percentage": lang_stats.shebang_percentage(),
							"files_detail": lang_stats.files_list().iter().map(|file| json!({
								"path": file.path(),
								"total_lines": file.total_lines(),
								"code_lines": file.code_lines(),
								"comment_lines": file.comment_lines(),
								"blank_lines": file.blank_lines(),
								"shebang_lines": file.shebang_lines(),
								"size": file.size(),
								"size_human": file.size_human(),
							})).collect::<Vec<_>>(),
						}),
					)
				})
				.collect();
			let output = json!({
				"analysis_path": path.display().to_string(),
				"summary": {
					"total_files": results.total_files(),
					"total_lines": results.total_lines(),
					"total_code_lines": results.total_code_lines(),
					"total_comment_lines": results.total_comment_lines(),
					"total_blank_lines": results.total_blank_lines(),
					"total_shebang_lines": results.total_shebang_lines(),
					"total_size": results.total_size(),
					"total_size_human": results.total_size_human(),
					"code_percentage": results.code_percentage(),
					"comment_percentage": results.comment_percentage(),
					"blank_percentage": results.blank_percentage(),
					"shebang_percentage": results.shebang_percentage(),
				},
				"languages": languages,
			});
			Ok(serde_json::to_string_pretty(&output)?)
		} else {
			Ok(serde_json::to_string_pretty(results)?)
		}
	}
}
