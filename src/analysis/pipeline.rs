use std::path::Path;

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
		let lang_name_lower = language.name.to_lowercase();
		return include_languages.iter().any(|filter| filter.to_lowercase() == lang_name_lower);
	}
	if !exclude_languages.is_empty() {
		let lang_name_lower = language.name.to_lowercase();
		return !exclude_languages.iter().any(|filter| filter.to_lowercase() == lang_name_lower);
	}
	true
}

/// Helper to create error context for metadata reading operations.
fn read_metadata_context(path: &Path) -> String {
	format!("Failed to read metadata for {}", path.display())
}

pub fn process_file(
	file_path: &Path,
	results: &mut AnalysisResults,
	collect_details: bool,
	include_languages: &[String],
	exclude_languages: &[String],
) -> Result<()> {
	let filename_os = file_path.file_name().context("Missing file name")?;
	let filename = filename_os.to_string_lossy();
	let metadata = file_path.metadata().with_context(|| read_metadata_context(file_path))?;
	let file_size = metadata.len();
	let language_from_name = langs::detect_language_info(&filename, None);
	if file_size == 0 {
		if let Some(language) = language_from_name {
			if !should_process_language(language, include_languages, exclude_languages) {
				return Ok(());
			}
			let contribution = FileContribution::new(0, 0, 0, 0, 0, file_size);
			let file_stats =
				collect_details.then(|| FileStats::new(file_path.display().to_string(), 0, 0, 0, 0, 0, file_size));
			results.add_file_stats(language, contribution, file_stats);
		}
		return Ok(());
	}
	let mut source = FileSource::open(file_path, file_size)?;
	let sample_bytes = source.sample(file_size)?;
	let Some((language, encoding)) = detect_language_and_encoding(&filename, &sample_bytes) else { return Ok(()) };
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
