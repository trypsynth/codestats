use std::{borrow::Cow, path::Path};

use anyhow::{Context as _, Result};

use super::{
	encoding::{FileEncoding, decode_bytes, detect_encoding, is_probably_binary},
	file_io::FileSource,
	stats::{AnalysisResults, FileContribution, FileStats},
};
use crate::langs::{self, Language};

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

/// Helper to create error context for metadata reading operations.
fn read_metadata_context(path: &Path) -> String {
	format!("Failed to read metadata for {}", path.display())
}

/// Analyze a single file and merge its statistics into `results`.
///
/// Returns an error for I/O or decoding failures.
pub fn process_file(
	file_path: &Path,
	results: &mut AnalysisResults,
	collect_details: bool,
	include_languages: &[String],
	exclude_languages: &[String],
) -> Result<()> {
	let filename_os = file_path.file_name().context("Missing file name")?;
	let filename_lossy = filename_os.to_string_lossy();
	let filename: Cow<'_, str> = if filename_lossy.contains('\u{FFFD}') {
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
			let contribution = FileContribution::new(0, 0, 0, 0, 0, file_size);
			let file_stats =
				collect_details.then(|| FileStats::new(file_path.display().to_string(), 0, 0, 0, 0, 0, file_size));
			results.add_file_stats(language, contribution, file_stats);
		} else {
			results.add_unrecognized_file();
		}
		return Ok(());
	}
	let mut source = FileSource::open(file_path, file_size)?;
	let sample_bytes = source.sample(file_size)?;
	let Some((language, encoding)) = detect_language_and_encoding(filename.as_ref(), &sample_bytes) else {
		results.add_unrecognized_file();
		return Ok(());
	};
	if !should_process_language(language, include_languages, exclude_languages) {
		return Ok(());
	}
	source.process(file_path, file_size, results, collect_details, language, encoding)
}

fn detect_language_from_samples(filename: &str, samples: &[u8], encoding: FileEncoding) -> Option<&'static Language> {
	if is_probably_binary(samples, encoding) {
		return None;
	}
	let decoded = decode_bytes(samples, encoding, true);
	langs::detect_language_info(filename, Some(decoded.as_ref()))
}

fn detect_language_and_encoding(filename: &str, samples: &[u8]) -> Option<(&'static Language, FileEncoding)> {
	let encoding = detect_encoding(samples);
	detect_language_from_samples(filename, samples, encoding).map(|language| (language, encoding))
}
