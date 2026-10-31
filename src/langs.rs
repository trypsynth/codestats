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
pub mod matchers;

pub use data::{LANGUAGES, Language};
pub use detection::{detect_language, get_candidates, is_shebang_for};

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
	let mut line_width = 0;
	for (i, lang) in LANGUAGES.iter().enumerate() {
		let separator = if i + 1 == LANGUAGES.len() { "." } else { ", " };
		let item_width = lang.name.chars().count() + separator.len();
		if line_width > 0 && line_width + item_width > terminal_width {
			writeln!(writer)?;
			line_width = 0;
		}
		write!(writer, "{}{separator}", lang.name)?;
		line_width += item_width;
	}
	if line_width > 0 {
		writeln!(writer)?;
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::get_candidates;

	#[test]
	fn get_candidates_uses_literal_map() {
		let candidates = get_candidates("Makefile");
		assert_eq!(candidates.len(), 1);
		assert_eq!(candidates[0].name, "Makefile");
	}

	#[test]
	fn get_candidates_matches_exact_filenames_case_sensitively() {
		assert!(!get_candidates("MAKEFILE").iter().any(|lang| lang.name == "Makefile"));
		// Bazel's `BUILD` must not claim an ordinary `build` script.
		assert!(!get_candidates("build").iter().any(|lang| lang.name == "Bazel"));
	}

	#[test]
	fn get_candidates_supports_wildcards() {
		let candidates = get_candidates("lib.rs");
		assert!(candidates.iter().any(|lang| lang.name == "Rust"));
	}

	#[test]
	fn get_candidates_matches_prefix_globs_case_sensitively() {
		// `Kconfig.*` is for `Kconfig.debug`, not Documentation/kconfig.rst.
		let candidates = get_candidates("kconfig.rst");
		assert!(!candidates.iter().any(|lang| lang.name == "Kconfig"));
	}

	#[test]
	fn get_candidates_matches_extensions_case_insensitively() {
		let candidates = get_candidates("LIB.RS");
		assert!(candidates.iter().any(|lang| lang.name == "Rust"));
	}

	use super::print_all_languages;

	#[test]
	fn print_all_languages_header() {
		let mut buf = Vec::new();
		print_all_languages(&mut buf, 80).unwrap();
		let output = String::from_utf8(buf).unwrap();
		let first_line = output.lines().next().unwrap();
		assert!(
			first_line.starts_with("Total number of supported programming"),
			"first line should start with the expected header, got: {first_line}"
		);
		// The header should contain the count of languages
		let count_str = super::data::LANGUAGES.len().to_string();
		assert!(
			first_line.contains(&count_str),
			"first line should contain the language count {count_str}, got: {first_line}"
		);
	}

	#[test]
	fn print_all_languages_ends_with_period() {
		let mut buf = Vec::new();
		print_all_languages(&mut buf, 80).unwrap();
		let output = String::from_utf8(buf).unwrap();
		let last_non_empty = output.lines().rfind(|l| !l.is_empty()).unwrap();
		assert!(last_non_empty.ends_with('.'), "last non-empty line should end with a period, got: {last_non_empty}");
	}

	#[test]
	fn print_all_languages_respects_width() {
		let width = 40;
		let mut buf = Vec::new();
		print_all_languages(&mut buf, width).unwrap();
		let output = String::from_utf8(buf).unwrap();
		// Skip the header line (line 0); only the language listing lines are wrapped
		for (i, line) in output.lines().enumerate().skip(1) {
			let char_count = line.chars().count();
			assert!(char_count <= width, "line {i} exceeds width {width} ({char_count} chars): {line}");
		}
	}

	#[test]
	fn print_all_languages_large_width() {
		let small_width = 40;
		let large_width = 10000;

		let mut buf_small = Vec::new();
		print_all_languages(&mut buf_small, small_width).unwrap();
		let output_small = String::from_utf8(buf_small).unwrap();

		let mut buf_large = Vec::new();
		print_all_languages(&mut buf_large, large_width).unwrap();
		let output_large = String::from_utf8(buf_large).unwrap();

		let lines_small = output_small.lines().count();
		let lines_large = output_large.lines().count();
		assert!(
			lines_large < lines_small,
			"large width should produce fewer lines ({lines_large}) than small width ({lines_small})"
		);
	}
}
