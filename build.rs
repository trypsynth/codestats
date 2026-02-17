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

type Result<T> = result::Result<T, Box<dyn Error>>;

fn validate_no_whitespace(s: &str, field: &str, idx: Option<usize>) -> result::Result<(), String> {
	if s.trim() == s {
		Ok(())
	} else {
		Err(idx.map_or_else(
			|| format!("{field}: has leading/trailing whitespace"),
			|i| format!("{field} {}: has leading/trailing whitespace", i + 1),
		))
	}
}

fn deserialize_vec_strings<'de, D>(
	deserializer: D,
	field: &'static str,
	allow_empty_vec: bool,
	validate: impl Fn(&str, usize) -> result::Result<(), String>,
) -> result::Result<Vec<String>, D::Error>
where
	D: Deserializer<'de>,
{
	let values: Vec<String> = Vec::deserialize(deserializer)?;
	if !allow_empty_vec && values.is_empty() {
		return Err(de::Error::custom(format!("{field} cannot be empty")));
	}
	for (idx, value) in values.iter().enumerate() {
		validate(value, idx).map_err(de::Error::custom)?;
	}
	Ok(values)
}

fn deserialize_file_patterns<'de, D>(deserializer: D) -> result::Result<Vec<String>, D::Error>
where
	D: Deserializer<'de>,
{
	deserialize_vec_strings(deserializer, "file_patterns", false, |s, idx| {
		if s.is_empty() {
			Err(format!("pattern {}: cannot be empty", idx + 1))
		} else {
			validate_no_whitespace(s, "pattern", Some(idx))
		}
	})
}

fn deserialize_line_comments<'de, D>(deserializer: D) -> result::Result<Vec<String>, D::Error>
where
	D: Deserializer<'de>,
{
	deserialize_vec_strings(deserializer, "line_comments", true, |s, idx| {
		if s.is_empty() { Err(format!("line comment {}: cannot be empty", idx + 1)) } else { Ok(()) }
	})
}

fn deserialize_block_comments<'de, D>(deserializer: D) -> result::Result<Vec<(String, String)>, D::Error>
where
	D: Deserializer<'de>,
{
	let pairs: Vec<Vec<String>> = Vec::deserialize(deserializer)?;
	let mut out: Vec<(String, String)> = Vec::with_capacity(pairs.len());
	for (idx, pair) in pairs.into_iter().enumerate() {
		let err = |msg| de::Error::custom(format!("block comment {}: {msg}", idx + 1));
		if pair.len() != 2 {
			return Err(err("must contain exactly start and end delimiters"));
		}
		let mut iter = pair.into_iter();
		let start = iter.next().unwrap();
		let end = iter.next().unwrap();
		if start.is_empty() {
			return Err(err("start cannot be empty"));
		}
		if end.is_empty() {
			return Err(err("end cannot be empty"));
		}
		out.push((start, end));
	}
	Ok(out)
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

fn write_field(output: &mut String, name: &str, value: impl std::fmt::Display) {
	let _ = writeln!(output, "\t\t{name}: {value},");
}

fn render_slice<T>(values: &[T], render: impl Fn(&T) -> String) -> String {
	if values.is_empty() {
		"&[]".to_string()
	} else {
		let joined = values.iter().map(render).collect::<Vec<_>>().join(", ");
		format!("&[{joined}]")
	}
}

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
		write_field(&mut output, "index", idx);
		write_field(&mut output, "name", format_args!("{:?}", lang.name));
		write_field(&mut output, "file_patterns", render_slice(&lang.file_patterns, |v| format!("{v:?}")));
		write_field(&mut output, "line_comments", render_slice(&lang.line_comments, |v| format!("{v:?}")));
		write_field(
			&mut output,
			"block_comments",
			render_slice(&lang.block_comments, |(s, e)| format!("({s:?}, {e:?})")),
		);
		write_field(&mut output, "nested_blocks", lang.nested_blocks);
		write_field(&mut output, "shebangs", render_slice(&lang.shebangs, |v| format!("{v:?}")));
		write_field(&mut output, "keywords", render_slice(&lang.keywords, |v| format!("{v:?}")));
		output.push_str("\t},\n");
	}
	output.push_str("];\n\n");
	output
}

fn normalize_languages(entries: IndexMap<String, LanguageConfig>) -> Result<Vec<LanguageConfig>> {
	let mut errors = Vec::new();
	let mut prev_name: Option<String> = None;
	let mut languages = Vec::with_capacity(entries.len());
	for (index, (name, mut config)) in entries.into_iter().enumerate() {
		if name.trim().is_empty() {
			errors.push(format!("Language at position {}: name cannot be empty", index + 1));
			continue;
		}
		if let Some(prev) = &prev_name
			&& name.to_lowercase() < prev.to_lowercase()
		{
			// Enforce a stable ordering so generated indices remain consistent across edits.
			errors.push(format!("Language '{name}' is not in alphabetical order (should come before '{prev}')"));
		}
		prev_name = Some(name.clone());
		config.name = name;
		languages.push(config);
	}
	if errors.is_empty() { Ok(languages) } else { Err(errors.join("\n").into()) }
}

struct PatternInfo {
	names: Vec<String>,
	all_have_keywords: bool,
}

struct LanguageValidator {
	errors: Vec<String>,
	seen_names: HashSet<String>,
	seen_patterns: HashMap<String, PatternInfo>,
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
				let info = self
					.seen_patterns
					.entry(pattern.clone())
					.or_insert(PatternInfo { names: Vec::new(), all_have_keywords: true });
				info.names.push(lang.name.clone());
				info.all_have_keywords &= !lang.keywords.is_empty();
			}
		}
		for (pattern, info) in &self.seen_patterns {
			if info.names.len() > 1 && !info.all_have_keywords {
				// Shared patterns need keyword disambiguation to avoid random selection.
				self.errors.push(format!(
					"Duplicate pattern '{}' in [{}] - all must have 'keywords' for disambiguation",
					pattern,
					info.names.join(", ")
				));
			}
		}
	}

	fn into_result(self) -> Result<()> {
		if self.errors.is_empty() {
			Ok(())
		} else {
			Err(format!("Language validation failed with {} error(s):\n{}", self.errors.len(), self.errors.join("\n"))
				.into())
		}
	}
}

fn validate_languages(languages: &[LanguageConfig]) -> Result<()> {
	let mut validator = LanguageValidator::new();
	validator.validate_all(languages);
	validator.into_result()
}

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
