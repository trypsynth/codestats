use std::{borrow::Cow, path::Path};

use anyhow::{Context as _, Result};

use super::{
	encoding::{decode_bytes, detect_encoding, is_probably_binary},
	file_io::FileSource,
	line_counter,
	stats::{AnalysisResults, LineCounts},
};
use crate::langs::{self, Language};

/// How much of a file to read when its extension is shared by several languages and its content decides.
const DETECTION_WINDOW: usize = 64 * 1024;

/// Check if a language should be processed based on include/exclude filters.
fn should_process_language(language: &Language, include_languages: &[String], exclude_languages: &[String]) -> bool {
	if !include_languages.is_empty() {
		return include_languages.iter().any(|filter| filter.eq_ignore_ascii_case(language.name));
	}
	if !exclude_languages.is_empty() {
		return !exclude_languages.iter().any(|filter| filter.eq_ignore_ascii_case(language.name));
	}
	true
}

/// Analyze a single file and merge its statistics into `results`.
///
/// Returns an error for I/O or decoding failures.
pub fn process_file(
	file_path: &Path,
	display_root: &Path,
	results: &mut AnalysisResults,
	collect_details: bool,
	include_languages: &[String],
	exclude_languages: &[String],
) -> Result<()> {
	let filename_os = file_path.file_name().context("Missing file name")?;
	let filename_lossy = filename_os.to_string_lossy();
	let filename: Cow<'_, str> = if filename_lossy.contains('\u{FFFD}') {
		// Fall back to a synthetic name so extension-based detection still works on non-UTF-8 filenames.
		file_path.extension().map_or_else(
			|| Cow::Borrowed(filename_lossy.as_ref()),
			|ext| {
				let ext_lossy = ext.to_string_lossy();
				Cow::Owned(format!("file.{ext_lossy}"))
			},
		)
	} else {
		Cow::Borrowed(filename_lossy.as_ref())
	};
	let metadata =
		file_path.metadata().with_context(|| format!("Failed to read metadata for {}", file_path.display()))?;
	let file_size = metadata.len();
	let candidates = langs::get_candidates(&filename);
	if let [language] = candidates[..]
		&& !should_process_language(language, include_languages, exclude_languages)
	{
		return Ok(());
	}
	if file_size == 0 {
		if let Some(language) = langs::detect_language(&candidates, None).language
			&& should_process_language(language, include_languages, exclude_languages)
		{
			let path = collect_details.then(|| display_path(file_path, display_root));
			results.add_file(language, &LineCounts::default(), 0, path);
		}
		return Ok(());
	}
	// A file whose name matches no language counts only if it starts with a shebang, so check that before reading it all.
	if candidates.is_empty() {
		let head = FileSource::head(file_path)?;
		let encoding = detect_encoding(&head);
		if is_probably_binary(&head, encoding)
			|| langs::detect_language(&[], Some(&decode_bytes(&head, encoding, true))).language.is_none()
		{
			return Ok(());
		}
	}
	let source = FileSource::open(file_path, file_size)?;
	let sample_bytes = source.sample();
	let encoding = detect_encoding(&sample_bytes);
	if is_probably_binary(&sample_bytes, encoding) {
		return Ok(());
	}
	// A single candidate decides by name alone, so only decode the sample when content has a say.
	let content_decides = candidates.len() != 1;
	let mut detection = langs::detect_language(
		&candidates,
		content_decides.then(|| decode_bytes(&sample_bytes, encoding, true)).as_deref(),
	);
	// When a shared extension was a close call on the small sample, look further: a long license or doc comment at the top of a file can drown out its code.
	if detection.close_call && source.len() > sample_bytes.len() {
		let wide_sample = &source[..source.len().min(DETECTION_WINDOW)];
		detection = langs::detect_language(&candidates, Some(&decode_bytes(wide_sample, encoding, true)));
	}
	let Some(language) = detection.language else {
		return Ok(());
	};
	if !should_process_language(language, include_languages, exclude_languages) {
		return Ok(());
	}
	let counts = line_counter::count_lines(language, encoding, &source);
	results.add_file(language, &counts, file_size, collect_details.then(|| display_path(file_path, display_root)));
	Ok(())
}

fn display_path(file_path: &Path, display_root: &Path) -> String {
	file_path.strip_prefix(display_root).unwrap_or(file_path).display().to_string()
}
