use std::{borrow::Cow, path::Path};

use anyhow::{Context as _, Result};

use super::{
	encoding::{decode_bytes, detect_encoding, is_probably_binary},
	file_io::FileSource,
	stats::{AnalysisResults, LineCounts},
};
use crate::langs::{self, Language};

/// Check if a language should be processed based on include/exclude filters.
/// How much of a file to read when its extension is shared by several languages and its content decides.
const DETECTION_WINDOW: usize = 64 * 1024;

fn should_process_language(language: &Language, include_languages: &[String], exclude_languages: &[String]) -> bool {
	if !include_languages.is_empty() {
		return include_languages.iter().any(|filter| filter.eq_ignore_ascii_case(language.name));
	}
	if !exclude_languages.is_empty() {
		return !exclude_languages.iter().any(|filter| filter.eq_ignore_ascii_case(language.name));
	}
	true
}

/// Helper to create error context for metadata reading operations.
fn read_metadata_context(path: &Path) -> String {
	format!("Failed to read metadata for {}", path.display())
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
	let metadata = file_path.metadata().with_context(|| read_metadata_context(file_path))?;
	let file_size = metadata.len();
	let language_from_name = langs::detect_language_info(filename.as_ref(), None);
	if file_size == 0 {
		if let Some(language) = language_from_name {
			if !should_process_language(language, include_languages, exclude_languages) {
				return Ok(());
			}
			let path = collect_details.then(|| display_path(file_path, display_root));
			results.add_file(language, &LineCounts::default(), 0, path);
		}
		return Ok(());
	}
	let mut source = FileSource::open(file_path, file_size)?;
	let sample_bytes = source.sample(file_size)?;
	let encoding = detect_encoding(&sample_bytes);
	if is_probably_binary(&sample_bytes, encoding) {
		return Ok(());
	}
	let mut detection = langs::detect_language(filename.as_ref(), Some(&decode_bytes(&sample_bytes, encoding, true)));
	// When a shared extension was a close call on the small sample, look further: a long license or doc comment at the top of a file can drown out its code.
	if detection.close_call && file_size > sample_bytes.len() as u64 {
		let wide_sample = source.head(DETECTION_WINDOW)?;
		detection = langs::detect_language(filename.as_ref(), Some(&decode_bytes(&wide_sample, encoding, true)));
	}
	let Some(language) = detection.language else {
		return Ok(());
	};
	if !should_process_language(language, include_languages, exclude_languages) {
		return Ok(());
	}
	let counts = source.count_lines(language, encoding)?;
	results.add_file(language, &counts, file_size, collect_details.then(|| display_path(file_path, display_root)));
	Ok(())
}

fn display_path(file_path: &Path, display_root: &Path) -> String {
	file_path.strip_prefix(display_root).unwrap_or(file_path).display().to_string()
}
