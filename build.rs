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
use serde::{Deserialize, Deserializer, de};

fn validate_no_whitespace(s: &str, field: &str, idx: Option<usize>) -> result::Result<(), String> {
	if s.trim() != s {
		let msg = if let Some(i) = idx {
			format!("{field} {}: has leading/trailing whitespace", i + 1)
		} else {
			format!("{field}: has leading/trailing whitespace")
		};
		Err(msg)
	} else {
		Ok(())
	}
}

fn deserialize_file_patterns<'de, D>(deserializer: D) -> result::Result<Vec<String>, D::Error>
where
	D: Deserializer<'de>,
{
	let patterns: Vec<String> = Vec::deserialize(deserializer)?;
	if patterns.is_empty() {
		return Err(de::Error::custom("file_patterns cannot be empty"));
	}
	for (idx, pattern) in patterns.iter().enumerate() {
		if pattern.is_empty() {
			return Err(de::Error::custom(format!("pattern {}: cannot be empty", idx + 1)));
		}
		validate_no_whitespace(pattern, "pattern", Some(idx)).map_err(de::Error::custom)?;
	}
	Ok(patterns)
}

fn deserialize_line_comments<'de, D>(deserializer: D) -> result::Result<Vec<String>, D::Error>
where
	D: Deserializer<'de>,
{
	let comments: Vec<String> = Vec::deserialize(deserializer)?;
	for (idx, comment) in comments.iter().enumerate() {
		if comment.is_empty() {
			return Err(de::Error::custom(format!("line comment {}: cannot be empty", idx + 1)));
		}
	}
	Ok(comments)
}

fn deserialize_block_comments<'de, D>(deserializer: D) -> result::Result<Vec<(String, String)>, D::Error>
where
	D: Deserializer<'de>,
{
	let pairs: Vec<Vec<String>> = Vec::deserialize(deserializer)?;
	let mut result = Vec::with_capacity(pairs.len());
	for (idx, pair) in pairs.into_iter().enumerate() {
		match pair.as_slice() {
			[start, end] if !start.is_empty() && !end.is_empty() => {
				result.push((start.clone(), end.clone()));
			}
			[start, _end] if start.is_empty() => {
				return Err(de::Error::custom(format!("block comment {}: start cannot be empty", idx + 1)));
			}
			[_, end] if end.is_empty() => {
				return Err(de::Error::custom(format!("block comment {}: end cannot be empty", idx + 1)));
			}
			[] => return Err(de::Error::custom(format!("block comment {}: start/end cannot be empty", idx + 1))),
			[_] => return Err(de::Error::custom(format!("block comment {}: missing end delimiter", idx + 1))),
			[_, _, ..] => {
				return Err(de::Error::custom(format!(
					"block comment {}: only start and end delimiters are supported",
					idx + 1
				)));
			}
		}
	}
	Ok(result)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct LanguageConfig {
	#[serde(skip)]
	name: String,
	#[serde(deserialize_with = "deserialize_file_patterns")]
	file_patterns: Vec<String>,
	#[serde(default, deserialize_with = "deserialize_line_comments")]
	line_comments: Vec<String>,
	#[serde(default, deserialize_with = "deserialize_block_comments")]
	block_comments: Vec<(String, String)>,
	#[serde(default)]
	nested_blocks: bool,
	#[serde(default)]
	shebangs: Vec<String>,
	#[serde(default)]
	keywords: Vec<String>,
}

const LANGUAGE_SCHEMA: &[(&str, &str)] = &[
	("index", "usize"),
	("name", "&'static str"),
	("file_patterns", "&'static [&'static str]"),
	("line_comments", "&'static [&'static str]"),
	("block_comments", "&'static [(&'static str, &'static str)]"),
	("nested_blocks", "bool"),
	("shebangs", "&'static [&'static str]"),
	("keywords", "&'static [&'static str]"),
];

fn render_languages(languages: &[LanguageConfig]) -> String {
	let mut output = String::new();
	output.push_str("#[derive(Debug, Clone, PartialEq, Eq)]\n");
	output.push_str("/// Holds information about a single programming language.\n");
	output.push_str("pub struct Language {\n");
	for (field, ty) in LANGUAGE_SCHEMA {
		let _ = writeln!(output, "\tpub {field}: {ty},");
	}
	output.push_str("}\n\n");
	output.push_str("pub static LANGUAGES: &[Language] = &[\n");
	for (idx, lang) in languages.iter().enumerate() {
		output.push_str("\tLanguage {\n");
		let _ = writeln!(output, "\t\tindex: {idx},");
		let _ = writeln!(output, "\t\tname: {:?},", lang.name);
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

fn render_str_slice(values: &[String]) -> String {
	if values.is_empty() {
		"&[]".to_string()
	} else {
		let joined = values.iter().map(|v| format!("{v:?}")).collect::<Vec<_>>().join(", ");
		format!("&[{joined}]")
	}
}

fn render_block_comments(values: &[(String, String)]) -> String {
	if values.is_empty() {
		"&[]".to_string()
	} else {
		let joined = values.iter().map(|(s, e)| format!("({s:?}, {e:?})")).collect::<Vec<_>>().join(", ");
		format!("&[{joined}]")
	}
}

fn normalize_languages(entries: IndexMap<String, LanguageConfig>) -> Result<Vec<LanguageConfig>> {
	let mut errors = Vec::new();
	let mut prev_name: Option<String> = None;
	let languages: Vec<LanguageConfig> = entries
		.into_iter()
		.enumerate()
		.filter_map(|(index, (name, mut config))| {
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
			config.name = name;
			Some(config)
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

	fn validate_all(&mut self, languages: &[LanguageConfig]) {
		for lang in languages {
			if !self.seen_names.insert(lang.name.clone()) {
				self.errors.push(format!("Duplicate language name '{}'", lang.name));
			}
			if lang.name.trim() != lang.name {
				self.errors.push(format!("Language '{}': name has leading/trailing whitespace", lang.name));
			}
			for pattern in &lang.file_patterns {
				self.seen_patterns.entry(pattern.clone()).or_default().push(lang.name.clone());
			}
		}
		self.validate_pattern_disambiguation(languages);
	}

	fn validate_pattern_disambiguation(&mut self, languages: &[LanguageConfig]) {
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

fn validate_languages(languages: &[LanguageConfig]) -> Result<()> {
	let mut validator = LanguageValidator::new();
	validator.validate_all(languages);
	validator.into_result()
}
