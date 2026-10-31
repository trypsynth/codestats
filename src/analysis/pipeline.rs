use std::{borrow::Cow, fs::File, io::Seek as _, path::Path};

use anyhow::{Context as _, Result};

use super::{
	encoding::{decode_bytes, detect_encoding, is_probably_binary},
	file_io::{self, FileSource},
	line_counter,
	stats::{AnalysisResults, LineCounts},
};
use crate::langs::{self, Language};

/// How much of a file to read when its extension is shared by several languages and its content decides.
const DETECTION_WINDOW: usize = 64 * 1024;

/// The `--lang` and `--exclude-lang` filters.
#[derive(Clone, Copy)]
pub struct LanguageFilter<'a> {
	pub include: &'a [String],
	pub exclude: &'a [String],
}

impl LanguageFilter<'_> {
	fn allows(self, language: &Language) -> bool {
		if !self.include.is_empty() {
			return self.include.iter().any(|filter| filter.eq_ignore_ascii_case(language.name));
		}
		!self.exclude.iter().any(|filter| filter.eq_ignore_ascii_case(language.name))
	}
}

/// The name to detect a file's language by. Non-UTF-8 names fall back to a synthetic one so extension-based detection still works.
fn detection_name(path: &Path) -> Option<Cow<'_, str>> {
	let name = path.file_name()?.to_string_lossy();
	if !name.contains('\u{FFFD}') {
		return Some(name);
	}
	Some(path.extension().map_or(name, |ext| Cow::Owned(format!("file.{}", ext.to_string_lossy()))))
}

/// The language and line counts of a file named `path` with contents `bytes`, or `None` if it isn't counted: unrecognized, binary, or filtered out.
pub fn count_bytes(path: &Path, bytes: &[u8], languages: LanguageFilter) -> Option<(&'static Language, LineCounts)> {
	count_contents(&langs::get_candidates(&detection_name(path)?), bytes, languages)
}

/// Whether a file named `path` could be counted, judging by its name alone: false when its only possible language is filtered out and a shebang couldn't claim it, so its contents needn't be read.
pub fn could_count(path: &Path, languages: LanguageFilter) -> bool {
	detection_name(path).is_some_and(|name| match langs::get_candidates(&name)[..] {
		[language] => languages.allows(language) || language.shebang_overrides,
		_ => true,
	})
}

/// Whether a file whose name says `language` might be a script its shebang claims instead: only where the language allows it, and only if it starts with `#!`.
fn may_be_script(language: &Language, bytes: &[u8]) -> bool {
	language.shebang_overrides && bytes.trim_ascii_start().starts_with(b"#!")
}

fn count_contents(
	candidates: &[&'static Language],
	bytes: &[u8],
	languages: LanguageFilter,
) -> Option<(&'static Language, LineCounts)> {
	if bytes.is_empty() {
		let language = langs::detect_language(candidates, None).language?;
		return languages.allows(language).then(|| (language, LineCounts::default()));
	}
	let sample_bytes = file_io::sample(bytes);
	let encoding = detect_encoding(&sample_bytes);
	if is_probably_binary(&sample_bytes, encoding) {
		return None;
	}
	// A single candidate decides by name alone, unless the file might be a script claimed by its shebang, so only decode the sample when content has a say.
	let content_decides = match candidates {
		[language] => may_be_script(language, &sample_bytes[encoding.bom_len.min(sample_bytes.len())..]),
		_ => true,
	};
	let mut detection = langs::detect_language(
		candidates,
		content_decides.then(|| decode_bytes(&sample_bytes, encoding, true)).as_deref(),
	);
	// When a shared extension was a close call on the small sample, look further: a long license or doc comment at the top of a file can drown out its code.
	if detection.close_call && bytes.len() > sample_bytes.len() {
		let wide_sample = &bytes[..bytes.len().min(DETECTION_WINDOW)];
		detection = langs::detect_language(candidates, Some(&decode_bytes(wide_sample, encoding, true)));
	}
	let language = detection.language.filter(|&language| languages.allows(language))?;
	Some((language, line_counter::count_lines(language, encoding, bytes)))
}

/// Analyze a single file and merge its statistics into `results`.
///
/// Returns an error for I/O or decoding failures.
pub fn process_file(
	file_path: &Path,
	display_root: &Path,
	results: &mut AnalysisResults,
	collect_details: bool,
	languages: LanguageFilter,
) -> Result<()> {
	let filename = detection_name(file_path).context("Missing file name")?;
	let candidates = langs::get_candidates(&filename);
	if let [language] = candidates[..]
		&& !languages.allows(language)
		&& !language.shebang_overrides
	{
		return Ok(());
	}
	let mut file = File::open(file_path).with_context(|| format!("Failed to open file {}", file_path.display()))?;
	// `fstat` on the open file rather than `stat` on its path, so the path is only looked up once.
	let file_size =
		file.metadata().with_context(|| format!("Failed to read metadata for {}", file_path.display()))?.len();
	// A file whose name matches no language, or only one that's filtered out, counts only if it starts with a shebang, so check that before reading it all.
	let needs_shebang = match candidates[..] {
		[] => true,
		[language] => !languages.allows(language),
		_ => false,
	};
	if needs_shebang && file_size > 0 {
		let head =
			FileSource::head(&mut file).with_context(|| format!("Failed to read file {}", file_path.display()))?;
		let encoding = detect_encoding(&head);
		if is_probably_binary(&head, encoding)
			|| langs::detect_language(&[], Some(&decode_bytes(&head, encoding, true))).language.is_none()
		{
			return Ok(());
		}
		file.rewind()?;
	}
	let source = FileSource::open(file, file_size, file_path)?;
	if let Some((language, counts)) = count_contents(&candidates, &source, languages) {
		results.add_file(language, &counts, file_size, collect_details.then(|| display_path(file_path, display_root)));
	}
	Ok(())
}

fn display_path(file_path: &Path, display_root: &Path) -> String {
	file_path.strip_prefix(display_root).unwrap_or(file_path).display().to_string()
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Counting a file's bytes in memory must give what processing the file on disk does, for every fixture.
	#[test]
	fn count_bytes_matches_process_file() {
		let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures");
		let mut checked = 0;
		let mut stack = vec![root.clone()];
		while let Some(dir) = stack.pop() {
			for entry in std::fs::read_dir(dir).unwrap() {
				let path = entry.unwrap().path();
				if path.is_dir() {
					stack.push(path);
					continue;
				}
				let languages = LanguageFilter { include: &[], exclude: &[] };
				let mut results = AnalysisResults::default();
				process_file(&path, &root, &mut results, true, languages).unwrap();
				let from_disk =
					results.languages().next().map(|(lang, stats)| (lang.name, *stats.files_list()[0].counts()));
				let from_bytes = count_bytes(&path, &std::fs::read(&path).unwrap(), languages)
					.map(|(lang, counts)| (lang.name, counts));
				assert_eq!(from_bytes, from_disk, "{}", path.display());
				checked += 1;
			}
		}
		assert!(checked > 100, "only {checked} fixtures");
	}
}
