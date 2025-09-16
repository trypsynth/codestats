use std::path::Path;

use anyhow::Result;
use serde_json::{Map, Value};

use super::OutputFormatter;
use crate::analysis::AnalysisResults;

/// JSON formatter
pub struct JsonFormatter;

impl OutputFormatter for JsonFormatter {
	fn format(&self, results: &AnalysisResults, path: &Path, verbose: bool) -> Result<String> {
		if verbose {
			let mut output = Map::new();
			output.insert("analysis_path".to_string(), Value::String(path.display().to_string()));
			let mut summary = Map::new();
			summary.insert("total_files".to_string(), Value::Number(results.total_files().into()));
			summary.insert("total_lines".to_string(), Value::Number(results.total_lines().into()));
			summary.insert("total_code_lines".to_string(), Value::Number(results.total_code_lines().into()));
			summary.insert("total_comment_lines".to_string(), Value::Number(results.total_comment_lines().into()));
			summary.insert("total_blank_lines".to_string(), Value::Number(results.total_blank_lines().into()));
			summary.insert("total_shebang_lines".to_string(), Value::Number(results.total_shebang_lines().into()));
			summary.insert("total_size".to_string(), Value::Number(results.total_size().into()));
			summary.insert(
				"total_size_human".to_string(),
				Value::String(human_bytes::human_bytes(crate::utils::size_to_f64(results.total_size()))),
			);
			summary.insert(
				"code_percentage".to_string(),
				Value::Number(serde_json::Number::from_f64(results.code_percentage()).unwrap_or_else(|| 0.into())),
			);
			summary.insert(
				"comment_percentage".to_string(),
				Value::Number(serde_json::Number::from_f64(results.comment_percentage()).unwrap_or_else(|| 0.into())),
			);
			summary.insert(
				"blank_percentage".to_string(),
				Value::Number(serde_json::Number::from_f64(results.blank_percentage()).unwrap_or_else(|| 0.into())),
			);
			summary.insert(
				"shebang_percentage".to_string(),
				Value::Number(serde_json::Number::from_f64(results.shebang_percentage()).unwrap_or_else(|| 0.into())),
			);
			output.insert("summary".to_string(), Value::Object(summary));
			let mut languages = Map::new();
			for (lang_name, lang_stats) in results.languages_by_lines() {
				let mut lang_obj = Map::new();
				lang_obj.insert("files".to_string(), Value::Number(lang_stats.files().into()));
				lang_obj.insert("lines".to_string(), Value::Number(lang_stats.lines().into()));
				lang_obj.insert("code_lines".to_string(), Value::Number(lang_stats.code_lines().into()));
				lang_obj.insert("comment_lines".to_string(), Value::Number(lang_stats.comment_lines().into()));
				lang_obj.insert("blank_lines".to_string(), Value::Number(lang_stats.blank_lines().into()));
				lang_obj.insert("shebang_lines".to_string(), Value::Number(lang_stats.shebang_lines().into()));
				lang_obj.insert("size".to_string(), Value::Number(lang_stats.size().into()));
				lang_obj.insert(
					"size_human".to_string(),
					Value::String(human_bytes::human_bytes(crate::utils::size_to_f64(lang_stats.size()))),
				);
				lang_obj.insert(
					"code_percentage".to_string(),
					Value::Number(
						serde_json::Number::from_f64(lang_stats.code_percentage()).unwrap_or_else(|| 0.into()),
					),
				);
				lang_obj.insert(
					"comment_percentage".to_string(),
					Value::Number(
						serde_json::Number::from_f64(lang_stats.comment_percentage()).unwrap_or_else(|| 0.into()),
					),
				);
				lang_obj.insert(
					"blank_percentage".to_string(),
					Value::Number(
						serde_json::Number::from_f64(lang_stats.blank_percentage()).unwrap_or_else(|| 0.into()),
					),
				);
				lang_obj.insert(
					"shebang_percentage".to_string(),
					Value::Number(
						serde_json::Number::from_f64(lang_stats.shebang_percentage()).unwrap_or_else(|| 0.into()),
					),
				);
				let files: Vec<Value> = lang_stats
					.files_list()
					.iter()
					.map(|file| {
						let mut file_obj = Map::new();
						file_obj.insert("path".to_string(), Value::String(file.path().to_string()));
						file_obj.insert("total_lines".to_string(), Value::Number(file.total_lines().into()));
						file_obj.insert("code_lines".to_string(), Value::Number(file.code_lines().into()));
						file_obj.insert("comment_lines".to_string(), Value::Number(file.comment_lines().into()));
						file_obj.insert("blank_lines".to_string(), Value::Number(file.blank_lines().into()));
						file_obj.insert("shebang_lines".to_string(), Value::Number(file.shebang_lines().into()));
						file_obj.insert("size".to_string(), Value::Number(file.size().into()));
						file_obj.insert(
							"size_human".to_string(),
							Value::String(human_bytes::human_bytes(crate::utils::size_to_f64(file.size()))),
						);
						Value::Object(file_obj)
					})
					.collect();
				lang_obj.insert("files_detail".to_string(), Value::Array(files));
				languages.insert(lang_name.clone(), Value::Object(lang_obj));
			}
			output.insert("languages".to_string(), Value::Object(languages));
			Ok(serde_json::to_string_pretty(&Value::Object(output))?)
		} else {
			Ok(serde_json::to_string_pretty(results)?)
		}
	}
}
