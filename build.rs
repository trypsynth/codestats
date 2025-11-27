#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use std::{
	collections::{BTreeMap, HashMap, HashSet},
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
	fn from_config(name: String, config: LanguageConfig) -> Self {
		let LanguageConfig { file_patterns, line_comments, block_comments, nested_blocks, shebangs, keywords } = config;
		let block_comments = block_comments
			.into_iter()
			.filter_map(|pair| {
				let mut iter = pair.into_iter();
				let start = iter.next()?;
				let end = iter.next()?;
				Some((start, end))
			})
			.collect();
		Self { name, file_patterns, line_comments, block_comments, nested_blocks, shebangs, keywords }
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

fn build_pattern_mappings(languages: &[ProcessedLanguage]) -> Vec<(String, Vec<usize>)> {
	let mut literal_map: BTreeMap<String, Vec<usize>> = BTreeMap::new();
	for (lang_idx, lang) in languages.iter().enumerate() {
		for pattern in &lang.file_patterns {
			if pattern.contains('*') {
				continue;
			}
			let indexes = literal_map.entry(pattern.clone()).or_default();
			if !indexes.contains(&lang_idx) {
				indexes.push(lang_idx);
			}
		}
	}
	literal_map.into_iter().collect()
}

fn render_languages(languages: &[ProcessedLanguage], pattern_mappings: &[(String, Vec<usize>)]) -> String {
	let mut output = String::new();
	output.push_str("use phf::{Map, phf_map};\n\n");
	output.push_str("#[derive(Debug, Clone, PartialEq, Eq)]\n");
	output.push_str("/// Holds information about a single programming language.\n");
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
	output.push_str("pub static PATTERN_MAP: Map<&'static str, &'static [&'static Language]> = phf_map! {\n");
	for (pattern, indexes) in pattern_mappings {
		let lang_refs = indexes.iter().map(|index| format!("&LANGUAGES[{index}]")).collect::<Vec<String>>().join(", ");
		let _ = writeln!(output, "\t{} => &[{lang_refs}],", render_str(pattern));
	}
	output.push_str("};\n");
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
					errors.push(format!(
						"Language '{name}' is not in alphabetical order (should come before '{prev}')"
					));
				}
			}
			prev_name = Some(name.clone());
			Some(ProcessedLanguage::from_config(name, config))
		})
		.collect();
	if errors.is_empty() {
		Ok(languages)
	} else {
		Err(errors.join("\n").into())
	}
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
	let pattern_mappings = build_pattern_mappings(&languages);
	let rendered = render_languages(&languages, &pattern_mappings);
	let out_dir = env::var("OUT_DIR")?;
	let dest_path = Path::new(&out_dir).join("languages.rs");
	fs::write(dest_path, rendered)?;
	println!("Generated language data for {} languages", languages.len());
	Ok(())
}

fn validate_languages(languages: &[ProcessedLanguage]) -> Result<()> {
	let mut errors = Vec::new();
	let seen_patterns = validate_language_fields(languages, &mut errors);
	validate_pattern_disambiguation(languages, seen_patterns, &mut errors);
	if !errors.is_empty() {
		let error_message =
			format!("Language validation failed with {} error(s):\n{}", errors.len(), errors.join("\n"));
		return Err(error_message.into());
	}
	Ok(())
}

fn validate_language_fields(languages: &[ProcessedLanguage], errors: &mut Vec<String>) -> HashMap<String, Vec<String>> {
	let mut seen_names: HashSet<String> = HashSet::new();
	let mut seen_patterns: HashMap<String, Vec<String>> = HashMap::new();
	for (index, lang) in languages.iter().enumerate() {
		let position = format!("Language at position {}", index + 1);
		validate_basic_fields(lang, &position, &mut seen_names, errors);
		validate_file_patterns(lang, &position, &mut seen_patterns, errors);
		validate_comment_fields(lang, &position, errors);
	}
	seen_patterns
}

fn validate_basic_fields(lang: &ProcessedLanguage, position: &str, seen_names: &mut HashSet<String>, errors: &mut Vec<String>) {
	if lang.name.is_empty() {
		errors.push(format!("{position}: 'name' field cannot be empty"));
	}
	if lang.file_patterns.is_empty() {
		errors.push(format!("{} ('{}'): 'file_patterns' field cannot be empty", position, lang.name));
	}
	if !seen_names.insert(lang.name.clone()) {
		errors.push(format!("{position}: Duplicate language name '{}'", lang.name));
	}
	if lang.name.trim() != lang.name {
		errors.push(format!("{} ('{}'): Language name has leading/trailing whitespace", position, lang.name));
	}
}

fn validate_file_patterns(
	lang: &ProcessedLanguage,
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

fn validate_comment_fields(lang: &ProcessedLanguage, position: &str, errors: &mut Vec<String>) {
	for (comment_idx, comment) in lang.line_comments.iter().enumerate() {
		if comment.is_empty() {
			errors.push(format!(
				"{} ('{}'), line comment {}: Line comment cannot be empty",
				position,
				lang.name,
				comment_idx + 1
			));
		}
	}
	for (comment_idx, block_comment) in lang.block_comments.iter().enumerate() {
		if block_comment.0.is_empty() {
			errors.push(format!(
				"{} ('{}'), block comment {}: Block comment start cannot be empty",
				position,
				lang.name,
				comment_idx + 1
			));
		}
		if block_comment.1.is_empty() {
			errors.push(format!(
				"{} ('{}'), block comment {}: Block comment end cannot be empty",
				position,
				lang.name,
				comment_idx + 1
			));
		}
	}
}

fn validate_pattern_disambiguation(
	languages: &[ProcessedLanguage],
	seen_patterns: HashMap<String, Vec<String>>,
	errors: &mut Vec<String>,
) {
	for (pattern, language_names) in seen_patterns {
		if language_names.len() > 1 {
			let all_have_keywords = language_names
				.iter()
				.all(|name| languages.iter().find(|l| l.name == *name).is_some_and(|l| !l.keywords.is_empty()));
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
