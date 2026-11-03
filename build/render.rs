//! Generates the Rust source for the static `LANGUAGES` table.

use std::{
	collections::BTreeMap,
	fmt::{Display, Write as _},
};

use crate::config::{LanguageConfig, RequireKeywords, StringSpec};

/// Fields of the generated `Language` struct, in order.
const LANGUAGE_FIELDS: &[(&str, &str)] = &[
	("index", "usize"),
	("name", "&'static str"),
	("file_patterns", "&'static [&'static str]"),
	("line_comments", "&'static [&'static str]"),
	// Each block comment is (start, end, nests).
	("block_comments", "&'static [(&'static str, &'static str, bool)]"),
	("column_one_comments", "&'static [&'static str]"),
	("shebangs", "&'static [&'static str]"),
	("keywords", "&'static [&'static str]"),
	("require_keywords", "RequireKeywords"),
	("case_sensitive_keywords", "bool"),
	("shebang_overrides", "bool"),
	("strings", "&'static [StringDelimiter]"),
	("c_preprocessor", "bool"),
	("regex_literals", "bool"),
	("line_splices", "bool"),
	// Block comments whose markers only count at the start of a line, or alone on it with `line_blocks_alone`.
	("line_block_comments", "&'static [(&'static str, &'static str)]"),
	("line_blocks_alone", "bool"),
	("embedding", "Option<Embedding>"),
	("heredoc", "Option<Heredoc>"),
];

/// Fields of the generated `StringDelimiter` struct, in order.
const STRING_DELIMITER_FIELDS: &[(&str, &str)] = &[
	("open", "&'static str"),
	("close", "&'static str"),
	("escape", "Option<u8>"),
	("multiline", "bool"),
	("char_literal", "bool"),
	("doc", "bool"),
	("delimited", "bool"),
	("attribute", "bool"),
	("interpolation", "Option<&'static str>"),
];

pub fn render_languages(languages: &[LanguageConfig]) -> String {
	let mut output = String::new();
	output.push_str(
		"#[expect(clippy::struct_excessive_bools, reason = \"independent switches from languages.json5\")]\n",
	);
	render_struct(
		&mut output,
		"StringDelimiter",
		"A string literal delimiter; comment markers inside strings are ignored.",
		"Debug, Clone, Copy, PartialEq, Eq",
		STRING_DELIMITER_FIELDS,
	);
	output.push_str(
		"#[expect(clippy::struct_excessive_bools, reason = \"independent per-language switches from languages.json5\")]\n",
	);
	render_struct(
		&mut output,
		"Language",
		"Holds information about a single programming language.",
		"Debug, Clone, PartialEq, Eq",
		LANGUAGE_FIELDS,
	);
	output.push_str("pub static LANGUAGES: &[Language] = &[\n");
	for (index, lang) in languages.iter().enumerate() {
		render_language(&mut output, index, lang);
	}
	output.push_str("];\n\n");
	render_pattern_tables(&mut output, languages);
	output
}

/// Render the tables that take a file name to its languages, so the program has nothing to build when it starts.
fn render_pattern_tables(output: &mut String, languages: &[LanguageConfig]) {
	let mut filenames: BTreeMap<String, Vec<usize>> = BTreeMap::new();
	let mut extensions: BTreeMap<String, Vec<usize>> = BTreeMap::new();
	let mut prefixes = Vec::new();
	for (index, lang) in languages.iter().enumerate() {
		for pattern in &lang.file_patterns {
			if let Some(prefix) = pattern.strip_suffix(".*") {
				prefixes.push((format!("{prefix}."), index));
			} else if let Some(extension) = pattern.strip_prefix("*.") {
				extensions.entry(extension.to_ascii_lowercase()).or_default().push(index);
			} else {
				filenames.entry(pattern.clone()).or_default().push(index);
			}
		}
	}
	render_table(output, "FILENAMES", &filenames);
	render_table(output, "EXTENSIONS", &extensions);
	let _ = writeln!(output, "pub static PREFIXES: &[(&str, u16)] = &{prefixes:?};");
}

/// A hash table with linear probing, in the layout `NameTable` reads. All its names are in one string, and its parts are plain numbers, so the loader has nothing in it to relocate.
fn render_table(output: &mut String, name: &str, entries: &BTreeMap<String, Vec<usize>>) {
	let mut names = String::new();
	let mut name_ends = Vec::new();
	let mut langs = Vec::new();
	let mut lang_ends = Vec::new();
	for (key, indexes) in entries {
		names.push_str(key);
		name_ends.push(names.len());
		langs.extend_from_slice(indexes);
		lang_ends.push(langs.len());
	}
	let mut slots = vec![0; (entries.len() * 2).next_power_of_two()];
	let mask = slots.len() - 1;
	for (entry, key) in entries.keys().enumerate() {
		let mut slot = name_hash(key) as usize & mask;
		while slots[slot] != 0 {
			slot = (slot + 1) & mask;
		}
		slots[slot] = entry + 1;
	}
	let _ = writeln!(
		output,
		"pub static {name}: NameTable = NameTable {{ names: {names:?}, name_ends: &{name_ends:?}, langs: &{langs:?}, lang_ends: &{lang_ends:?}, slots: &{slots:?} }};"
	);
}

/// FNV-1a, as `NameTable` computes it at run time.
fn name_hash(name: &str) -> u32 {
	name.bytes().fold(0x811c_9dc5, |hash, byte| (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193))
}

fn render_struct(output: &mut String, name: &str, doc: &str, derives: &str, fields: &[(&str, &str)]) {
	let _ = writeln!(output, "#[derive({derives})]\n/// {doc}\npub struct {name} {{");
	for (field, ty) in fields {
		let _ = writeln!(output, "\tpub {field}: {ty},");
	}
	output.push_str("}\n\n");
}

fn render_language(output: &mut String, index: usize, lang: &LanguageConfig) {
	let block_comments = render_slice(&lang.block_comments, |(start, end)| {
		format!("({start:?}, {end:?}, {})", lang.nested_blocks.nests(start))
	});
	output.push_str("\tLanguage {\n");
	write_field(output, "index", index);
	write_field(output, "name", format_args!("{:?}", lang.name));
	write_field(output, "file_patterns", quoted_slice(&lang.file_patterns));
	write_field(output, "line_comments", quoted_slice(&lang.line_comments));
	write_field(output, "block_comments", block_comments);
	write_field(output, "column_one_comments", quoted_slice(&lang.column_one_comments));
	write_field(output, "shebangs", quoted_slice(&lang.shebangs));
	write_field(output, "keywords", quoted_slice(lang.keywords.words()));
	let require = match lang.keywords.required() {
		None => "Never",
		Some(RequireKeywords::Shared) => "Shared",
		Some(RequireKeywords::Always) => "Always",
	};
	write_field(output, "require_keywords", format_args!("RequireKeywords::{require}"));
	write_field(output, "case_sensitive_keywords", lang.keywords.case_sensitive());
	write_field(output, "shebang_overrides", lang.shebang_overrides);
	write_field(output, "strings", render_slice(&lang.strings, render_string_delimiter));
	write_field(output, "c_preprocessor", lang.c_preprocessor);
	write_field(output, "regex_literals", lang.regex_literals);
	write_field(output, "line_splices", lang.line_splices);
	write_field(
		output,
		"line_block_comments",
		render_slice(&lang.line_block_comments, |(start, end)| format!("({start:?}, {end:?})")),
	);
	write_field(output, "line_blocks_alone", lang.line_blocks_alone);
	write_field(
		output,
		"embedding",
		lang.embedding.map_or_else(|| "None".to_string(), |e| format!("Some(Embedding::{e:?})")),
	);
	write_field(
		output,
		"heredoc",
		lang.heredoc.map_or_else(|| "None".to_string(), |h| format!("Some(Heredoc::{h:?})")),
	);
	output.push_str("\t},\n");
}

fn render_string_delimiter(spec: &StringSpec) -> String {
	let config = spec.config();
	let escape = config.escape.as_ref().map_or_else(|| "None".to_string(), |e| format!("Some({:?})", e.as_bytes()[0]));
	let interpolation =
		config.interpolation.as_ref().map_or_else(|| "None".to_string(), |hole| format!("Some({hole:?})"));
	format!(
		"StringDelimiter {{ open: {:?}, close: {:?}, escape: {escape}, multiline: {}, char_literal: {}, doc: {}, delimited: {}, attribute: {}, interpolation: {interpolation} }}",
		config.open,
		config.close(),
		config.multiline,
		config.char_literal,
		config.doc,
		config.delimited,
		config.attribute
	)
}

fn write_field(output: &mut String, name: &str, value: impl Display) {
	let _ = writeln!(output, "\t\t{name}: {value},");
}

fn quoted_slice(values: &[String]) -> String {
	render_slice(values, |v| format!("{v:?}"))
}

fn render_slice<T>(values: &[T], render: impl Fn(&T) -> String) -> String {
	if values.is_empty() {
		"&[]".to_string()
	} else {
		format!("&[{}]", values.iter().map(render).collect::<Vec<_>>().join(", "))
	}
}
