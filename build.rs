#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use std::{
	collections::{HashMap, HashSet},
	env,
	error::Error,
	fmt::Write,
	fs,
	path::Path,
	result,
};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
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
	#[serde(default)]
	keywords: Option<Vec<String>>,
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

impl From<Language> for ProcessedLanguage {
	fn from(lang: Language) -> Self {
		Self {
			name: lang.name,
			file_patterns: lang.file_patterns,
			line_comments: lang.line_comments.unwrap_or_default(),
			block_comments: lang
				.block_comments
				.unwrap_or_default()
				.into_iter()
				.filter_map(|pair| if pair.len() == 2 { Some((pair[0].clone(), pair[1].clone())) } else { None })
				.collect(),
			nested_blocks: lang.nested_blocks.unwrap_or(false),
			shebangs: lang.shebangs.unwrap_or_default(),
			keywords: lang.keywords.unwrap_or_default(),
		}
	}
}

fn get_language_schema() -> Vec<(&'static str, &'static str)> {
	vec![
		("name", "&'static str"),
		("file_patterns", "&'static [&'static str]"),
		("line_comments", "&'static [&'static str]"),
		("block_comments", "&'static [(&'static str, &'static str)]"),
		("nested_blocks", "bool"),
		("shebangs", "&'static [&'static str]"),
		("keywords", "&'static [&'static str]"),
	]
}

fn build_pattern_mappings(languages: &[ProcessedLanguage]) -> Vec<(String, usize)> {
	let mut pattern_mappings = HashMap::new();
	for (lang_idx, lang) in languages.iter().enumerate() {
		for pattern in &lang.file_patterns {
			pattern_mappings.insert(pattern.clone(), lang_idx);
		}
	}
	pattern_mappings.into_iter().collect()
}

fn render_languages(languages: &[ProcessedLanguage], pattern_mappings: &[(String, usize)]) -> String {
	let mut output = String::new();
	output.push_str("use phf::{Map, phf_map};\n\n");
	output.push_str("#[derive(Debug, Clone, PartialEq, Eq)]\n");
	output.push_str("pub struct Language {\n");
	for (field, ty) in get_language_schema() {
		let _ = writeln!(output, "\tpub {field}: {ty},");
	}
	output.push_str("}\n\n");
	output.push_str("pub static LANGUAGES: &[Language] = &[\n");
	for lang in languages {
		output.push_str("\tLanguage {\n");
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
	output.push_str("pub static LANGUAGE_MAP: Map<&'static str, &Language> = phf_map! {\n");
	for (index, lang) in languages.iter().enumerate() {
		let _ = writeln!(output, "\t{} => &LANGUAGES[{index}],", render_str(&lang.name));
	}
	output.push_str("};\n\n");
	output.push_str("pub static PATTERN_MAP: Map<&'static str, &Language> = phf_map! {\n");
	for (pattern, index) in pattern_mappings {
		let _ = writeln!(output, "\t{} => &LANGUAGES[{index}],", render_str(pattern));
	}
	output.push_str("};\n");
	output
}

fn render_str(value: &str) -> String {
	format!("\"{}\"", escape_rust_string(value))
}

fn render_str_slice(values: &[String]) -> String {
	if values.is_empty() {
		"&[]".to_string()
	} else {
		let joined = values.iter().map(|value| render_str(value)).collect::<Vec<_>>().join(", ");
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
			.collect::<Vec<_>>()
			.join(", ");
		format!("&[{joined}]")
	}
}

type Result<T> = result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
	println!("cargo:rerun-if-changed=languages.json");
	let manifest_dir = env::var("CARGO_MANIFEST_DIR")?;
	let json_path = Path::new(&manifest_dir).join("languages.json");
	let json_content = fs::read_to_string(&json_path)?;
	let languages: Vec<Language> = serde_json::from_str(&json_content)?;
	validate_languages(&languages)?;
	let processed_languages: Vec<ProcessedLanguage> = languages.into_iter().map(ProcessedLanguage::from).collect();
	let pattern_mappings = build_pattern_mappings(&processed_languages);
	let rendered = render_languages(&processed_languages, &pattern_mappings);
	let out_dir = env::var("OUT_DIR")?;
	let dest_path = Path::new(&out_dir).join("languages.rs");
	fs::write(dest_path, rendered)?;
	println!("Generated language data for {} languages", processed_languages.len());
	Ok(())
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
	validate_alphabetical_order(languages, &mut errors);
	let seen_patterns = validate_language_fields(languages, &mut errors);
	validate_pattern_disambiguation(languages, seen_patterns, &mut errors);
	if !errors.is_empty() {
		let error_message =
			format!("Language validation failed with {} error(s):\n{}", errors.len(), errors.join("\n"));
		return Err(error_message.into());
	}
	Ok(())
}

fn validate_alphabetical_order(languages: &[Language], errors: &mut Vec<String>) {
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
}

fn validate_language_fields(languages: &[Language], errors: &mut Vec<String>) -> HashMap<String, Vec<String>> {
	let mut seen_names = HashSet::new();
	let mut seen_patterns: HashMap<String, Vec<String>> = HashMap::new();
	for (index, lang) in languages.iter().enumerate() {
		let position = format!("Language at position {}", index + 1);
		validate_basic_fields(lang, &position, &mut seen_names, errors);
		validate_file_patterns(lang, &position, &mut seen_patterns, errors);
		validate_comment_fields(lang, &position, errors);
	}
	seen_patterns
}

fn validate_basic_fields(lang: &Language, position: &str, seen_names: &mut HashSet<String>, errors: &mut Vec<String>) {
	if lang.name.is_empty() {
		errors.push(format!("{position}: 'name' field cannot be empty"));
	}
	if lang.file_patterns.is_empty() {
		errors.push(format!("{} ('{}'): 'file_patterns' field cannot be empty", position, lang.name));
	}
	if !seen_names.insert(lang.name.clone()) {
		errors.push(format!("{}: Duplicate language name '{}'", position, lang.name));
	}
	if lang.name.trim() != lang.name {
		errors.push(format!("{} ('{}'): Language name has leading/trailing whitespace", position, lang.name));
	}
}

fn validate_file_patterns(
	lang: &Language,
	position: &str,
	seen_patterns: &mut HashMap<String, Vec<String>>,
	errors: &mut Vec<String>,
) {
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
		seen_patterns.entry(pattern.clone()).or_default().push(lang.name.clone());
	}
}

fn validate_comment_fields(lang: &Language, position: &str, errors: &mut Vec<String>) {
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

fn validate_pattern_disambiguation(
	languages: &[Language],
	seen_patterns: HashMap<String, Vec<String>>,
	errors: &mut Vec<String>,
) {
	for (pattern, language_names) in seen_patterns {
		if language_names.len() > 1 {
			let all_have_keywords = language_names.iter().all(|name| {
				languages
					.iter()
					.find(|l| l.name == *name)
					.and_then(|l| l.keywords.as_ref())
					.is_some_and(|k| !k.is_empty())
			});
			if !all_have_keywords {
				errors.push(format!(
					"Duplicate pattern '{}' in [{}] - all must have 'keywords' for disambiguation",
					pattern,
					language_names.join(", ")
				));
			}
		}
	}
}
