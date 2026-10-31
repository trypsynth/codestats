//! Checks across fields and languages that serde can't express.

use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;

use crate::{
	Result,
	config::{LanguageConfig, NestedBlocks},
};

/// Turn the parsed map into a list, checking names and enforcing alphabetical order so generated indices stay stable across edits.
pub fn normalize_languages(entries: IndexMap<String, LanguageConfig>) -> Result<Vec<LanguageConfig>> {
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
			errors.push(format!("Language '{name}' is not in alphabetical order (should come before '{prev}')"));
		}
		prev_name = Some(name.clone());
		config.name = name;
		languages.push(config);
	}
	if errors.is_empty() { Ok(languages) } else { Err(errors.join("\n").into()) }
}

pub fn validate_languages(languages: &[LanguageConfig]) -> Result<()> {
	let mut validator = Validator::default();
	for lang in languages {
		validator.check_name(lang);
		validator.check_block_comments(lang);
		validator.check_strings(lang);
		validator.check_keywords(lang);
		validator.record_patterns(lang);
	}
	validator.check_shared_patterns();
	if validator.errors.is_empty() {
		Ok(())
	} else {
		Err(format!(
			"Language validation failed with {} error(s):\n{}",
			validator.errors.len(),
			validator.errors.join("\n")
		)
		.into())
	}
}

#[derive(Default)]
struct Validator {
	errors: Vec<String>,
	seen_names: HashSet<String>,
	/// The languages using each file pattern, and whether they all have keywords.
	patterns: HashMap<String, (Vec<String>, bool)>,
}

impl Validator {
	fn error(&mut self, lang: &LanguageConfig, message: impl std::fmt::Display) {
		self.errors.push(format!("Language '{}': {message}", lang.name));
	}

	fn check_name(&mut self, lang: &LanguageConfig) {
		if !self.seen_names.insert(lang.name.clone()) {
			self.errors.push(format!("Duplicate language name '{}'", lang.name));
		}
		if lang.name.trim() != lang.name {
			self.error(lang, "name has leading/trailing whitespace");
		}
	}

	fn check_block_comments(&mut self, lang: &LanguageConfig) {
		if let NestedBlocks::Starts(starts) = &lang.nested_blocks {
			for start in starts {
				if !lang.block_comments.iter().any(|(s, _)| s == start) {
					self.error(lang, format_args!("nested_blocks lists '{start}', which isn't a block comment start"));
				}
			}
		}
		for (start, end) in &lang.block_comments {
			// `/**` ... `*/` is already covered by `/*` ... `*/`, and as a longer start it would swallow `/**/`.
			if lang.block_comments.iter().any(|(s, e)| s != start && start.starts_with(s.as_str()) && e == end) {
				self.error(
					lang,
					format_args!("block comment '{start}' is redundant with a shorter start that has the same end"),
				);
			}
			// Identical delimiters can't distinguish opening a nested comment from closing one.
			if start == end && lang.nested_blocks.nests(start) {
				self.error(
					lang,
					format_args!("block comment '{start}' can't nest because its start and end are identical"),
				);
			}
		}
	}

	fn check_strings(&mut self, lang: &LanguageConfig) {
		for spec in &lang.strings {
			let config = spec.config();
			if config.open.is_empty() || config.close().is_empty() {
				self.error(lang, "string delimiters cannot be empty");
			}
			if config.escape.as_ref().is_some_and(|e| e.len() != 1 || !e.is_ascii()) {
				self.error(lang, "string escape must be a single ASCII character");
			}
			if config.char_literal && (config.open.len() != 1 || config.close() != config.open) {
				self.error(lang, "char literals need a single-character delimiter");
			}
			if lang.line_comments.contains(&config.open) || lang.block_comments.iter().any(|(s, _)| s == &config.open) {
				self.error(lang, format_args!("string delimiter '{}' is also a comment marker", config.open));
			}
		}
	}

	fn check_keywords(&mut self, lang: &LanguageConfig) {
		if lang.require_keywords && lang.keywords.is_empty() {
			self.error(lang, "require_keywords needs keywords");
		}
		for (i, keyword) in lang.keywords.iter().enumerate() {
			if lang.keywords[..i].contains(keyword) {
				self.error(lang, format!("duplicate keyword {keyword:?}"));
			}
		}
	}

	fn record_patterns(&mut self, lang: &LanguageConfig) {
		for pattern in &lang.file_patterns {
			let (names, all_have_keywords) = self.patterns.entry(pattern.clone()).or_insert((Vec::new(), true));
			names.push(lang.name.clone());
			*all_have_keywords &= !lang.keywords.is_empty();
		}
	}

	/// Languages sharing a pattern are told apart by keywords, so each of them needs some.
	fn check_shared_patterns(&mut self) {
		for (pattern, (names, all_have_keywords)) in &self.patterns {
			if names.len() > 1 && !all_have_keywords {
				self.errors.push(format!(
					"Duplicate pattern '{pattern}' in [{}] - all must have 'keywords' for disambiguation",
					names.join(", ")
				));
			}
		}
	}
}
