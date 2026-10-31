//! The shape of `languages.json5`, with per-field checks done while deserializing.

use std::result;

use serde::{Deserialize, Deserializer, de};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
#[expect(clippy::struct_excessive_bools, reason = "independent per-language switches from languages.json5")]
pub struct LanguageConfig {
	#[serde(skip)]
	pub name: String,
	#[serde(deserialize_with = "deserialize_file_patterns")]
	pub file_patterns: Vec<String>,
	#[serde(default, deserialize_with = "deserialize_line_comments")]
	pub line_comments: Vec<String>,
	#[serde(default, deserialize_with = "deserialize_block_comments")]
	pub block_comments: Vec<(String, String)>,
	#[serde(default)]
	pub nested_blocks: NestedBlocks,
	#[serde(default, deserialize_with = "deserialize_line_comments")]
	pub column_one_comments: Vec<String>,
	#[serde(default)]
	pub shebangs: Vec<String>,
	#[serde(default)]
	pub keywords: Vec<String>,
	#[serde(default)]
	pub require_keywords: bool,
	#[serde(default)]
	pub case_sensitive_keywords: bool,
	#[serde(default)]
	pub shebang_overrides: bool,
	#[serde(default)]
	pub strings: Vec<StringSpec>,
	#[serde(default)]
	pub c_preprocessor: bool,
	#[serde(default)]
	pub regex_literals: bool,
	#[serde(default)]
	pub embedding: Option<Embedding>,
}

/// How a language embeds others; mirrors `Embedding` in `src/langs/data.rs`.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Embedding {
	Html,
	Astro,
	Markdown,
}

/// Which block comment pairs nest: `true` for all of them, or a list of the start delimiters that do.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum NestedBlocks {
	All(bool),
	Starts(Vec<String>),
}

impl Default for NestedBlocks {
	fn default() -> Self {
		Self::All(false)
	}
}

impl NestedBlocks {
	pub fn nests(&self, start: &str) -> bool {
		match self {
			Self::All(all) => *all,
			Self::Starts(starts) => starts.iter().any(|s| s == start),
		}
	}
}

/// A string literal delimiter. The short form `"\""` opens and closes with the same text, uses `\` as its escape, and stays on one line.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum StringSpec {
	Short(String),
	Full(StringConfig),
}

impl StringSpec {
	pub fn config(&self) -> StringConfig {
		match self {
			Self::Short(open) => StringConfig {
				open: open.clone(),
				close: None,
				escape: default_escape(),
				multiline: false,
				char_literal: false,
				doc: false,
			},
			Self::Full(config) => config.clone(),
		}
	}
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StringConfig {
	pub open: String,
	close: Option<String>,
	/// `null` means the string has no escape character (raw strings).
	#[serde(default = "default_escape")]
	pub escape: Option<String>,
	#[serde(default)]
	pub multiline: bool,
	/// A character literal like C's `'a'`: only treated as a string when it looks like one, so Rust lifetimes (`'a`) stay code.
	#[serde(default, rename = "char")]
	pub char_literal: bool,
	/// A docstring delimiter (Python's `"""`): when it starts a line, its contents count as comment rather than code.
	#[serde(default)]
	pub doc: bool,
}

impl StringConfig {
	/// The closing delimiter, which defaults to the opening one.
	pub fn close(&self) -> &str {
		self.close.as_deref().unwrap_or(&self.open)
	}
}

#[expect(clippy::unnecessary_wraps, reason = "serde's `default` must match the field type, Option<String>")]
fn default_escape() -> Option<String> {
	Some("\\".to_string())
}

fn deserialize_file_patterns<'de, D>(deserializer: D) -> result::Result<Vec<String>, D::Error>
where
	D: Deserializer<'de>,
{
	deserialize_vec_strings(deserializer, "file_patterns", false, |s, idx| {
		if s.is_empty() {
			Err(format!("pattern {}: cannot be empty", idx + 1))
		} else if s.trim() != s {
			Err(format!("pattern {}: has leading/trailing whitespace", idx + 1))
		} else {
			Ok(())
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
	pairs
		.into_iter()
		.enumerate()
		.map(|(idx, pair)| {
			let err = |msg| de::Error::custom(format!("block comment {}: {msg}", idx + 1));
			let [start, end]: [String; 2] =
				pair.try_into().map_err(|_| err("must contain exactly start and end delimiters"))?;
			if start.is_empty() {
				return Err(err("start cannot be empty"));
			}
			if end.is_empty() {
				return Err(err("end cannot be empty"));
			}
			Ok((start, end))
		})
		.collect()
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
