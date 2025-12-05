#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use std::{
	collections::{HashMap, HashSet},
	env,
	error::Error,
	fmt::Write as _,
	fs,
	path::Path,
	result,
};

use indexmap::IndexMap;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LanguageConfig {
	file_patterns: Vec<String>,
	#[serde(default)]
	line_comments: Vec<String>,
	#[serde(default)]
	block_comments: Vec<Vec<String>>,
	#[serde(default)]
	nested_blocks: bool,
	#[serde(default)]
	shebangs: Vec<String>,
	#[serde(default)]
	keywords: Vec<String>,
}

#[derive(Debug, Clone)]
struct ProcessedLanguage {
	name: String,
	file_patterns: Vec<String>,
	line_comments: Vec<String>,
	block_comments: Vec<(String, String)>,
	nested_blocks: bool,
	shebangs: Vec<String>,
	keywords: Vec<String>,
}

impl ProcessedLanguage {
	fn from_config(name: String, config: LanguageConfig) -> result::Result<Self, String> {
		let LanguageConfig { file_patterns, line_comments, block_comments, nested_blocks, shebangs, keywords } = config;
		let mut errors = Vec::new();
		let mut parsed_block_comments = Vec::new();
		for (idx, pair) in block_comments.into_iter().enumerate() {
			match pair.as_slice() {
				[start, end] => parsed_block_comments.push((start.clone(), end.clone())),
				[] => errors.push(format!("Language '{name}', block comment {}: start/end cannot be empty", idx + 1)),
				[_single] => {
					errors.push(format!("Language '{name}', block comment {}: missing end delimiter", idx + 1));
				}
				[_, _, ..] => errors.push(format!(
					"Language '{name}', block comment {}: only start and end delimiters are supported",
					idx + 1
				)),
			}
		}
		if errors.is_empty() {
			Ok(Self {
				name,
				file_patterns,
				line_comments,
				block_comments: parsed_block_comments,
				nested_blocks,
				shebangs,
				keywords,
			})
		} else {
			Err(errors.join("\n"))
		}
	}
}

fn get_language_schema() -> Vec<(&'static str, &'static str)> {
	vec![
		("index", "usize"),
		("name", "&'static str"),
		("file_patterns", "&'static [&'static str]"),
		("line_comments", "&'static [&'static str]"),
		("block_comments", "&'static [(&'static str, &'static str)]"),
		("nested_blocks", "bool"),
		("shebangs", "&'static [&'static str]"),
		("keywords", "&'static [&'static str]"),
	]
}

fn render_languages(languages: &[ProcessedLanguage]) -> String {
	let mut output = String::new();
	output.push_str("#[derive(Debug, Clone, PartialEq, Eq)]\n");
	output.push_str("/// Holds information about a single programming language.\n");
	output.push_str("pub struct Language {\n");
	for (field, ty) in get_language_schema() {
		let _ = writeln!(output, "\tpub {field}: {ty},");
	}
	output.push_str("}\n\n");
	output.push_str("pub static LANGUAGES: &[Language] = &[\n");
	for (idx, lang) in languages.iter().enumerate() {
		output.push_str("\tLanguage {\n");
		let _ = writeln!(output, "\t\tindex: {idx},");
		let _ = writeln!(output, "\t\tname: {},", render_str(&lang.name));
		let _ = writeln!(output, "\t\tfile_patterns: {},", render_str_slice(&lang.file_patterns));
		let _ = writeln!(output, "\t\tline_comments: {},", render_str_slice(&lang.line_comments));
		let _ = writeln!(output, "\t\tblock_comments: {},", render_block_comments(&lang.block_comments));
		let _ = writeln!(output, "\t\tnested_blocks: {},", lang.nested_blocks);
		let _ = writeln!(output, "\t\tshebangs: {},", render_str_slice(&lang.shebangs));
		let _ = writeln!(output, "\t\tkeywords: {},", render_str_slice(&lang.keywords));
		output.push_str("\t},\n");
	}
	output.push_str("];\n\n");
	output
}

fn render_str(value: &str) -> String {
	format!("{value:?}")
}

fn render_str_slice(values: &[String]) -> String {
	if values.is_empty() {
		"&[]".to_string()
	} else {
		let joined = values.iter().map(|value| render_str(value)).collect::<Vec<String>>().join(", ");
		format!("&[{joined}]")
	}
}

fn render_block_comments(values: &[(String, String)]) -> String {
	if values.is_empty() {
		"&[]".to_string()
	} else {
		let joined = values
			.iter()
			.map(|(start, end)| format!("({}, {})", render_str(start), render_str(end)))
			.collect::<Vec<String>>()
			.join(", ");
		format!("&[{joined}]")
	}
}

fn normalize_languages(entries: IndexMap<String, LanguageConfig>) -> Result<Vec<ProcessedLanguage>> {
	let mut errors = Vec::new();
	let mut prev_name: Option<String> = None;
	let languages: Vec<ProcessedLanguage> = entries
		.into_iter()
		.enumerate()
		.filter_map(|(index, (name, config))| {
			if name.trim().is_empty() {
				errors.push(format!("Language at position {}: name cannot be empty", index + 1));
				return None;
			}
			if let Some(prev) = &prev_name {
				if name.to_lowercase() < prev.to_lowercase() {
					errors
						.push(format!("Language '{name}' is not in alphabetical order (should come before '{prev}')"));
				}
			}
			prev_name = Some(name.clone());
			match ProcessedLanguage::from_config(name, config) {
				Ok(lang) => Some(lang),
				Err(err) => {
					errors.push(err);
					None
				}
			}
		})
		.collect();
	if errors.is_empty() { Ok(languages) } else { Err(errors.join("\n").into()) }
}

type Result<T> = result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
	let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
	let json_path = Path::new(&manifest_dir).join("languages.json5");
	println!("cargo:rerun-if-changed={}", json_path.display());
	let json_content = fs::read_to_string(&json_path)?;
	let language_map: IndexMap<String, LanguageConfig> =
		json5::from_str(&json_content).map_err(|e| format!("Failed to parse languages.json5: {e}"))?;
	let languages = normalize_languages(language_map)?;
	validate_languages(&languages)?;
	let rendered = render_languages(&languages);
	let out_dir = env::var("OUT_DIR")?;
	let dest_path = Path::new(&out_dir).join("languages.rs");
	fs::write(dest_path, rendered)?;
	println!("Generated language data for {} languages", languages.len());
	Ok(())
}

struct LanguageValidator {
	errors: Vec<String>,
	seen_names: HashSet<String>,
	seen_patterns: HashMap<String, Vec<String>>,
}

impl LanguageValidator {
	fn new() -> Self {
		Self { errors: Vec::new(), seen_names: HashSet::new(), seen_patterns: HashMap::new() }
	}

	fn validate_all(&mut self, languages: &[ProcessedLanguage]) {
		for (index, lang) in languages.iter().enumerate() {
			let position = format!("Language at position {}", index + 1);
			self.validate_language(lang, &position);
		}
		self.validate_pattern_disambiguation(languages);
	}

	fn validate_language(&mut self, lang: &ProcessedLanguage, position: &str) {
		self.validate_name(lang, position);
		self.validate_file_patterns(lang, position);
		self.validate_comments(lang, position);
	}

	fn validate_name(&mut self, lang: &ProcessedLanguage, position: &str) {
		if lang.name.is_empty() {
			self.errors.push(format!("{position}: 'name' field cannot be empty"));
		}
		if !self.seen_names.insert(lang.name.clone()) {
			self.errors.push(format!("{position}: Duplicate language name '{}'", lang.name));
		}
		if lang.name.trim() != lang.name {
			self.errors.push(format!("{} ('{}'): Language name has leading/trailing whitespace", position, lang.name));
		}
		if lang.file_patterns.is_empty() {
			self.errors.push(format!("{} ('{}'): 'file_patterns' field cannot be empty", position, lang.name));
		}
	}

	fn validate_file_patterns(&mut self, lang: &ProcessedLanguage, position: &str) {
		for (pattern_idx, pattern) in lang.file_patterns.iter().enumerate() {
			if pattern.is_empty() {
				self.errors.push(format!(
					"{} ('{}'), pattern {}: File pattern cannot be empty",
					position,
					lang.name,
					pattern_idx + 1
				));
			}
			if pattern.trim() != pattern {
				self.errors.push(format!(
					"{} ('{}'), pattern {}: File pattern '{}' has leading/trailing whitespace",
					position,
					lang.name,
					pattern_idx + 1,
					pattern
				));
			}
			self.seen_patterns.entry(pattern.clone()).or_default().push(lang.name.clone());
		}
	}

	fn validate_comments(&mut self, lang: &ProcessedLanguage, position: &str) {
		for (comment_idx, comment) in lang.line_comments.iter().enumerate() {
			if comment.is_empty() {
				self.errors.push(format!(
					"{} ('{}'), line comment {}: Line comment cannot be empty",
					position,
					lang.name,
					comment_idx + 1
				));
			}
		}
		for (comment_idx, block_comment) in lang.block_comments.iter().enumerate() {
			if block_comment.0.is_empty() {
				self.errors.push(format!(
					"{} ('{}'), block comment {}: Block comment start cannot be empty",
					position,
					lang.name,
					comment_idx + 1
				));
			}
			if block_comment.1.is_empty() {
				self.errors.push(format!(
					"{} ('{}'), block comment {}: Block comment end cannot be empty",
					position,
					lang.name,
					comment_idx + 1
				));
			}
		}
	}

	fn validate_pattern_disambiguation(&mut self, languages: &[ProcessedLanguage]) {
		for (pattern, language_names) in &self.seen_patterns {
			if language_names.len() > 1 {
				let all_have_keywords = language_names
					.iter()
					.all(|name| languages.iter().find(|l| &l.name == name).is_some_and(|l| !l.keywords.is_empty()));
				if !all_have_keywords {
					self.errors.push(format!(
						"Duplicate pattern '{}' in [{}] - all must have 'keywords' for disambiguation",
						pattern,
						language_names.join(", ")
					));
				}
			}
		}
	}

	fn into_result(self) -> Result<()> {
		if self.errors.is_empty() {
			Ok(())
		} else {
			let error_message =
				format!("Language validation failed with {} error(s):\n{}", self.errors.len(), self.errors.join("\n"));
			Err(error_message.into())
		}
	}
}

fn validate_languages(languages: &[ProcessedLanguage]) -> Result<()> {
	let mut validator = LanguageValidator::new();
	validator.validate_all(languages);
	validator.into_result()
}
