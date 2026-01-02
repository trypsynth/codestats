//! Language detection and definition management.
//!
//! ## Language Detection Strategy
//!
//! The detection process follows a multi-stage approach:
//!
//! 1. File pattern matching: Match filename against patterns (e.g., `*.rs` = Rust and `CMakeLists.txt` = `CMake`).
//! 2. Disambiguation: When multiple languages match, use file content analysis. Check for shebang lines and score based on comment style matches and keyword occurrences.
//! 3. Specialized handling: Symbol-only languages such as Brainfuck use custom detection to avoid being detected as B overly permissivly.
//!
//! ## Language Definitions
//!
//! Language metadata is loaded from `languages.json5` at build time and compiled into static data structures. See [`LANGUAGES`] for the complete list.

use std::io::Write;

use anyhow::Result;

mod data;
mod detection;

pub use data::{LANGUAGES, Language};
pub use detection::{detect_language_info, scoring};

use crate::display::formatting::pluralize;

/// Write a list of all supported programming languages to a writer.
/// # Errors
///
/// Returns an error if writing to the provided writer fails.
pub fn print_all_languages(writer: &mut dyn Write, terminal_width: usize) -> Result<()> {
	let lang_count = u64::try_from(LANGUAGES.len()).unwrap_or(u64::MAX);
	writeln!(
		writer,
		"Total number of supported programming {}: {}",
		pluralize(lang_count, "language", "languages"),
		LANGUAGES.len()
	)?;
	let mut lines: Vec<String> = Vec::new();
	let mut current_line = String::new();
	for (i, lang) in LANGUAGES.iter().enumerate() {
		let is_last = i == LANGUAGES.len() - 1;
		let separator = if is_last { "." } else { ", " };
		let item = format!("{}{}", lang.name, separator);
		let would_exceed = !current_line.is_empty() && current_line.len() + item.len() > terminal_width;
		if would_exceed {
			lines.push(current_line);
			current_line = item;
		} else {
			current_line.push_str(&item);
		}
	}
	if !current_line.is_empty() {
		lines.push(current_line);
	}
	for line in lines {
		writeln!(writer, "{line}")?;
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::detection::patterns::get_candidates;

	#[test]
	fn get_candidates_uses_literal_map() {
		let candidates = get_candidates("Makefile");
		assert_eq!(candidates.len(), 1);
		assert_eq!(candidates[0].name, "Makefile");
	}

	#[test]
	fn get_candidates_handles_case_insensitive_literals() {
		let candidates = get_candidates("MAKEFILE");
		assert_eq!(candidates.len(), 1);
		assert_eq!(candidates[0].name, "Makefile");
	}

	#[test]
	fn get_candidates_supports_wildcards() {
		let candidates = get_candidates("lib.rs");
		assert!(candidates.iter().any(|lang| lang.name == "Rust"));
	}
}
