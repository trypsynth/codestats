//! Generates the Rust source for the static `LANGUAGES` table.

use std::fmt::{Display, Write as _};

use crate::config::{LanguageConfig, StringSpec};

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
	("require_keywords", "bool"),
	("case_sensitive_keywords", "bool"),
	("strings", "&'static [StringDelimiter]"),
	("c_preprocessor", "bool"),
	("regex_literals", "bool"),
];

/// Fields of the generated `StringDelimiter` struct, in order.
const STRING_DELIMITER_FIELDS: &[(&str, &str)] = &[
	("open", "&'static str"),
	("close", "&'static str"),
	("escape", "Option<u8>"),
	("multiline", "bool"),
	("char_literal", "bool"),
	("doc", "bool"),
];

pub fn render_languages(languages: &[LanguageConfig]) -> String {
	let mut output = String::new();
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
	output.push_str("];\n");
	output
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
	write_field(output, "keywords", quoted_slice(&lang.keywords));
	write_field(output, "require_keywords", lang.require_keywords);
	write_field(output, "case_sensitive_keywords", lang.case_sensitive_keywords);
	write_field(output, "strings", render_slice(&lang.strings, render_string_delimiter));
	write_field(output, "c_preprocessor", lang.c_preprocessor);
	write_field(output, "regex_literals", lang.regex_literals);
	output.push_str("\t},\n");
}

fn render_string_delimiter(spec: &StringSpec) -> String {
	let config = spec.config();
	let escape = config.escape.as_ref().map_or_else(|| "None".to_string(), |e| format!("Some({:?})", e.as_bytes()[0]));
	format!(
		"StringDelimiter {{ open: {:?}, close: {:?}, escape: {escape}, multiline: {}, char_literal: {}, doc: {} }}",
		config.open,
		config.close(),
		config.multiline,
		config.char_literal,
		config.doc
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
