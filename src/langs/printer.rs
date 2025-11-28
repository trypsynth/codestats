use std::io::Write;

use anyhow::Result;

use crate::{langs::LANGUAGES, utils::pluralize};

/// Write a list of all supported programming languages to a writer.
/// # Errors
///
/// Returns an error if writing to the provided writer fails.
pub fn print_all_languages(writer: &mut dyn Write) -> Result<()> {
	let lang_count = u64::try_from(LANGUAGES.len()).unwrap_or(u64::MAX);
	writeln!(
		writer,
		"Total number of supported programming {}: {}",
		pluralize(lang_count, "language", "languages"),
		LANGUAGES.len()
	)?;
	let last_idx = LANGUAGES.len().saturating_sub(1);
	for (i, lang) in LANGUAGES.iter().enumerate() {
		let suffix = if i == last_idx { "." } else { "," };
		writeln!(writer, "{}{suffix}", lang.name)?;
	}
	Ok(())
}
