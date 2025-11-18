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

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Language {
	name: String,
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

impl From<Language> for ProcessedLanguage {
	fn from(lang: Language) -> Self {
		let Language { name, file_patterns, line_comments, block_comments, nested_blocks, shebangs, keywords } = lang;
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

type Result<T> = result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
	let manifest_dir: String = env::var("CARGO_MANIFEST_DIR")?;
	let json_path = Path::new(&manifest_dir).join("languages.jsonc");
	println!("cargo:rerun-if-changed={}", json_path.display());
	let json_content: String = fs::read_to_string(&json_path)?;
	let json_content = normalize_jsonc(&json_content);
	let languages: Vec<Language> = serde_json::from_str(&json_content)?;
	validate_languages(&languages)?;
	let processed_languages: Vec<ProcessedLanguage> = languages.into_iter().map(ProcessedLanguage::from).collect();
	let pattern_mappings = build_pattern_mappings(&processed_languages);
	let rendered = render_languages(&processed_languages, &pattern_mappings);
	let out_dir: String = env::var("OUT_DIR")?;
	let dest_path = Path::new(&out_dir).join("languages.rs");
	fs::write(dest_path, rendered)?;
	println!("Generated language data for {} languages", processed_languages.len());
	Ok(())
}

fn validate_languages(languages: &[Language]) -> Result<()> {
	let mut errors: Vec<String> = Vec::new();
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

fn validate_basic_fields(lang: &Language, position: &str, seen_names: &mut HashSet<String>, errors: &mut Vec<String>) {
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
	for (comment_idx, comment_pair) in lang.block_comments.iter().enumerate() {
		match comment_pair.as_slice() {
			[start, end] => {
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
			}
			_ => {
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

fn normalize_jsonc(input: &str) -> String {
	let without_comments = strip_jsonc(input);
	remove_trailing_commas(&without_comments)
}

fn strip_jsonc(input: &str) -> String {
	let mut output = String::with_capacity(input.len());
	let mut chars = input.chars().peekable();
	let mut in_string = false;
	let mut escape = false;
	let mut line_comment = false;
	let mut block_comment = false;
	while let Some(c) = chars.next() {
		if line_comment {
			if c == '\n' {
				line_comment = false;
				output.push('\n');
			} else if c == '\r' {
				line_comment = false;
				output.push('\r');
			}
			continue;
		}
		if block_comment {
			if c == '*' && chars.peek().is_some_and(|next| *next == '/') {
				chars.next();
				block_comment = false;
			} else if c == '\n' || c == '\r' {
				output.push(c);
			}
			continue;
		}
		if in_string {
			output.push(c);
			if escape {
				escape = false;
			} else if c == '\\' {
				escape = true;
			} else if c == '"' {
				in_string = false;
			}
			continue;
		}
		match c {
			'"' => {
				in_string = true;
				output.push(c);
			}
			'/' => match chars.peek() {
				Some('/') => {
					line_comment = true;
					chars.next();
				}
				Some('*') => {
					block_comment = true;
					chars.next();
				}
				_ => output.push('/'),
			},
			_ => output.push(c),
		}
	}
	output
}

fn remove_trailing_commas(input: &str) -> String {
	struct PendingComma {
		idx: usize,
		whitespace: String,
	}
	let mut output = String::with_capacity(input.len());
	let mut in_string = false;
	let mut escape = false;
	let mut pending_comma: Option<PendingComma> = None;
	for c in input.chars() {
		if in_string {
			output.push(c);
			if escape {
				escape = false;
			} else if c == '\\' {
				escape = true;
			} else if c == '"' {
				in_string = false;
			}
			continue;
		}
		if let Some(mut pending) = pending_comma.take() {
			if c.is_whitespace() {
				pending.whitespace.push(c);
				pending_comma = Some(pending);
				continue;
			}
			if c == ']' || c == '}' {
				output.truncate(pending.idx);
				output.push_str(&pending.whitespace);
				output.push(c);
				continue;
			}
			output.push_str(&pending.whitespace);
		}
		match c {
			'"' => {
				in_string = true;
				output.push(c);
			}
			',' => {
				let idx = output.len();
				output.push(',');
				pending_comma = Some(PendingComma { idx, whitespace: String::new() });
			}
			_ => output.push(c),
		}
	}
	output
}
