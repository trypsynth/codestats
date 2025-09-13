#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use std::{
	collections::{HashMap, HashSet},
	env,
	error::Error,
	fs,
	path::Path,
	result,
};

use serde::{Deserialize, Serialize};
use tera::{Context, Tera, Value, to_value};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
struct Language {
	name: String,
	file_patterns: Vec<String>,
	#[serde(default)]
	line_comments: Option<Vec<String>>,
	#[serde(default)]
	block_comments: Option<Vec<Vec<String>>>,
	#[serde(default)]
	nested_blocks: Option<bool>,
	#[serde(default)]
	shebangs: Option<Vec<String>>,
}

fn get_language_schema() -> Vec<(&'static str, &'static str)> {
	vec![
		("name", "&'static str"),
		("file_patterns", "&'static [&'static str]"),
		("line_comments", "&'static [&'static str]"),
		("block_comments", "&'static [(&'static str, &'static str)]"),
		("nested_blocks", "bool"),
		("shebangs", "&'static [&'static str]"),
	]
}

type Result<T> = result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
	println!("cargo:rerun-if-changed=src/languages.json");
	println!("cargo:rerun-if-changed=templates/languages.rs");
	let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
	let json_path = Path::new(&manifest_dir).join("src/languages.json");
	let json_content = fs::read_to_string(&json_path)?;
	let languages: Vec<Language> = serde_json::from_str(&json_content)?;
	validate_languages(&languages)?;
	let processed_languages: Vec<Language> = languages
		.into_iter()
		.map(|lang| Language {
			name: lang.name,
			file_patterns: lang.file_patterns,
			line_comments: Some(lang.line_comments.unwrap_or_default()),
			block_comments: Some(lang.block_comments.unwrap_or_default()),
			nested_blocks: Some(lang.nested_blocks.unwrap_or(false)),
			shebangs: Some(lang.shebangs.unwrap_or_default()),
		})
		.collect();
	let mut pattern_mappings = HashMap::new();
	for (lang_idx, lang) in processed_languages.iter().enumerate() {
		for pattern in &lang.file_patterns {
			pattern_mappings.insert(pattern.clone(), lang_idx);
		}
	}
	let pattern_mappings: Vec<(String, usize)> = pattern_mappings.into_iter().collect();
	let mut tera = Tera::new("templates/**/*")?;
	tera.register_filter("rust_string", rust_string_filter);
	tera.register_filter("field_render", field_render_filter);
	let mut context = Context::new();
	context.insert("languages", &to_value(&processed_languages)?);
	context.insert("pattern_mappings", &to_value(pattern_mappings)?);
	context.insert("struct_fields", &to_value(get_language_schema())?);
	let rendered = tera.render("languages.rs", &context)?;
	let out_dir = env::var("OUT_DIR")?;
	let dest_path = Path::new(&out_dir).join("languages.rs");
	fs::write(dest_path, rendered)?;
	println!("Generated language data for {} languages using Tera", processed_languages.len());
	Ok(())
}

fn rust_string_filter(value: &Value, _: &HashMap<String, Value>) -> tera::Result<Value> {
	match value {
		Value::String(s) => Ok(Value::String(format!("\"{}\"", escape_rust_string(s)))),
		_ => Err("rust_string filter can only be used on strings".into()),
	}
}

fn field_render_filter(value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
	let field_type =
		args.get("field_type").and_then(|v| v.as_str()).ok_or("field_render requires field_type argument")?;
	match field_type {
		"&'static str" => {
			if let Value::String(s) = value {
				Ok(Value::String(format!("\"{}\"", escape_rust_string(s))))
			} else {
				Err("Expected string for &'static str field".into())
			}
		}
		"&'static [&'static str]" => {
			if let Value::Array(arr) = value {
				let rendered: result::Result<Vec<String>, tera::Error> = arr
					.iter()
					.map(|v| {
						if let Value::String(s) = v {
							Ok(format!("\"{}\"", escape_rust_string(s)))
						} else {
							Err("Expected string in array".into())
						}
					})
					.collect();
				match rendered {
					Ok(strings) => Ok(Value::String(format!("&[{}]", strings.join(", ")))),
					Err(e) => Err(e),
				}
			} else {
				Err("Expected array for &'static [&'static str] field".into())
			}
		}
		"&'static [(&'static str, &'static str)]" => {
			if let Value::Array(arr) = value {
				let rendered: result::Result<Vec<String>, tera::Error> = arr
					.iter()
					.map(|v| {
						if let Value::Array(pair) = v {
							if pair.len() == 2 {
								if let (Value::String(a), Value::String(b)) = (&pair[0], &pair[1]) {
									Ok(format!("(\"{}\", \"{}\")", escape_rust_string(a), escape_rust_string(b)))
								} else {
									Err("Expected string pair".into())
								}
							} else {
								Err("Expected pair (2 elements) in array".into())
							}
						} else {
							Err("Expected array pairs".into())
						}
					})
					.collect();
				match rendered {
					Ok(strings) => Ok(Value::String(format!("&[{}]", strings.join(", ")))),
					Err(e) => Err(e),
				}
			} else {
				Err("Expected array for block comments field".into())
			}
		}
		"bool" => Ok(value.clone()),
		_ => Err(format!("Unknown field type: {field_type}").into()),
	}
}

fn escape_rust_string(s: &str) -> String {
	s.chars()
		.map(|c| match c {
			'\\' => "\\\\".to_string(),
			'"' => "\\\"".to_string(),
			'\n' => "\\n".to_string(),
			'\r' => "\\r".to_string(),
			'\t' => "\\t".to_string(),
			c => c.to_string(),
		})
		.collect()
}

fn validate_languages(languages: &[Language]) -> Result<()> {
	let mut errors = Vec::new();
	let mut seen_names = HashSet::new();
	let mut prev_name: Option<&str> = None;
	for lang in languages {
		if let Some(prev) = prev_name {
			if lang.name.to_lowercase() < prev.to_lowercase() {
				errors.push(format!(
					"Language '{}' is not in alphabetical order (should come before '{}')",
					lang.name, prev
				));
			}
		}
		prev_name = Some(&lang.name);
	}
	for (index, lang) in languages.iter().enumerate() {
		let position = format!("Language at position {}", index + 1);
		if lang.name.is_empty() {
			errors.push(format!("{position}: 'name' field cannot be empty"));
		}
		if lang.file_patterns.is_empty() {
			errors.push(format!("{} ('{}'): 'file_patterns' field cannot be empty", position, lang.name));
		}
		if !seen_names.insert(&lang.name) {
			errors.push(format!("{}: Duplicate language name '{}'", position, lang.name));
		}
		if lang.name.trim() != lang.name {
			errors.push(format!("{} ('{}'): Language name has leading/trailing whitespace", position, lang.name));
		}
		for (pattern_idx, pattern) in lang.file_patterns.iter().enumerate() {
			if pattern.is_empty() {
				errors.push(format!(
					"{} ('{}'), pattern {}: File pattern cannot be empty",
					position,
					lang.name,
					pattern_idx + 1
				));
			}
			if pattern.trim() != pattern {
				errors.push(format!(
					"{} ('{}'), pattern {}: File pattern '{}' has leading/trailing whitespace",
					position,
					lang.name,
					pattern_idx + 1,
					pattern
				));
			}
		}
		if let Some(ref line_comments) = lang.line_comments {
			for (comment_idx, comment) in line_comments.iter().enumerate() {
				if comment.is_empty() {
					errors.push(format!(
						"{} ('{}'), line comment {}: Line comment cannot be empty",
						position,
						lang.name,
						comment_idx + 1
					));
				}
			}
		}
		if let Some(ref block_comments) = lang.block_comments {
			for (comment_idx, comment_pair) in block_comments.iter().enumerate() {
				if comment_pair.len() == 2 {
    					let (start, end) = (&comment_pair[0], &comment_pair[1]);
    					if start.is_empty() {
    						errors.push(format!(
    							"{} ('{}'), block comment {}: Block comment start cannot be empty",
    							position,
    							lang.name,
    							comment_idx + 1
    						));
    					}
    					if end.is_empty() {
    						errors.push(format!(
    							"{} ('{}'), block comment {}: Block comment end cannot be empty",
    							position,
    							lang.name,
    							comment_idx + 1
    						));
    					}
    				} else {
    					errors.push(format!(
    						"{} ('{}'), block comment {}: Block comment must have exactly 2 elements (start, end)",
    						position,
    						lang.name,
    						comment_idx + 1
    					));
    				}
			}
		}
	}
	if !errors.is_empty() {
		let error_message =
			format!("Language validation failed with {} error(s):\n{}", errors.len(), errors.join("\n"));
		return Err(error_message.into());
	}
	Ok(())
}
