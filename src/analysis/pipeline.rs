use std::{borrow::Cow, fs::File, io::Seek as _, path::Path};

use anyhow::{Context as _, Result};

use super::{
	attributes::{Attributes, DirCache},
	encoding::{decode_bytes, detect_encoding, is_probably_binary},
	file_io::{self, FileSource},
	generated::{has_generated_header, is_generated_name},
	line_counter,
	stats::{AnalysisResults, FileCounts},
};
use crate::langs::{self, Language};

/// How much of a file to read when its extension is shared by several languages and its content decides.
const DETECTION_WINDOW: usize = 64 * 1024;

/// Which files count: the `--lang` and `--exclude-lang` filters, `--include-generated`, and the project's `.gitattributes`.
#[derive(Clone, Copy)]
pub struct FileFilter<'a> {
	pub include: &'a [String],
	pub exclude: &'a [String],
	pub include_generated: bool,
	pub attributes: Option<&'a Attributes>,
}

impl FileFilter<'_> {
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

/// The languages a file named `name` at `path` could be, and the filter its contents are judged by, or `None` when its name and attributes leave it out: vendored or documentation, or generated and not included. A file whose attributes say it isn't generated is counted whatever its name or header say, and one whose attributes name its language is that language.
fn judge_name<'a>(
	path: &Path,
	name: &str,
	languages: FileFilter<'a>,
	cache: &mut DirCache,
) -> Option<(Vec<&'static Language>, FileFilter<'a>)> {
	let verdict = languages.attributes.and_then(|attributes| attributes.verdict(path, cache)).unwrap_or_default();
	let generated = verdict.generated.unwrap_or_else(|| is_generated_name(name));
	if verdict.excluded || (generated && !languages.include_generated) {
		return None;
	}
	let include_generated = languages.include_generated || verdict.generated == Some(false);
	let candidates = verdict.language.map_or_else(|| langs::get_candidates(name), |language| vec![language]);
	Some((candidates, FileFilter { include_generated, ..languages }))
}

/// The language and line counts of a file named `path` with contents `bytes`, or `None` if it isn't counted: unrecognized, binary, or filtered out.
pub fn count_bytes(
	path: &Path,
	bytes: &[u8],
	languages: FileFilter,
	cache: &mut DirCache,
) -> Option<(&'static Language, FileCounts)> {
	let (candidates, languages) = judge_name(path, &detection_name(path)?, languages, cache)?;
	count_contents(&candidates, bytes, languages)
}

/// Whether a file named `path` could be counted, judging by its name and attributes alone: false when they leave it out, or its only possible language is filtered out and a shebang couldn't claim it, so its contents needn't be read.
pub fn could_count(path: &Path, languages: FileFilter, cache: &mut DirCache) -> bool {
	detection_name(path).and_then(|name| judge_name(path, &name, languages, cache)).is_some_and(
		|(candidates, languages)| match candidates[..] {
			[language] => languages.allows(language) || language.shebang_overrides,
			_ => true,
		},
	)
}

/// Whether a file whose name says `language` might be a script its shebang claims instead: only where the language allows it, and only if it starts with `#!`.
fn may_be_script(language: &Language, bytes: &[u8]) -> bool {
	language.shebang_overrides && bytes.trim_ascii_start().starts_with(b"#!")
}

fn count_contents(
	candidates: &[&'static Language],
	bytes: &[u8],
	languages: FileFilter,
) -> Option<(&'static Language, FileCounts)> {
	if bytes.is_empty() {
		let language = langs::detect_language(candidates, None).language?;
		return languages.allows(language).then(|| (language, FileCounts::default()));
	}
	let sample_bytes = file_io::sample(bytes);
	let encoding = detect_encoding(&sample_bytes);
	if is_probably_binary(&sample_bytes, encoding) || (!languages.include_generated && has_generated_header(bytes)) {
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
	languages: FileFilter,
	cache: &mut DirCache,
) -> Result<()> {
	let filename = detection_name(file_path).context("Missing file name")?;
	let Some((candidates, languages)) = judge_name(file_path, &filename, languages, cache) else {
		return Ok(());
	};
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
	// A file whose name matches no language, or only ones that are filtered out, counts only if it starts with a shebang, so check that before reading it all, or weighing its content between candidates that can't count anyway.
	let needs_shebang = !candidates.iter().any(|&language| languages.allows(language));
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
		results.add_file(language, counts, file_size, collect_details.then(|| display_path(file_path, display_root)));
	}
	Ok(())
}

fn display_path(file_path: &Path, display_root: &Path) -> String {
	// `--git` joins a lone file to `.`, so leading `./`s are left out of the comparison.
	let without_dot = |path: &Path| path.strip_prefix(".").map_or_else(|_| path.to_path_buf(), Path::to_path_buf);
	let file = without_dot(file_path);
	let relative = file.strip_prefix(without_dot(display_root)).unwrap_or(file_path);
	// Counting a single file makes it the root, which would leave nothing of its path.
	let relative =
		if relative.as_os_str().is_empty() { file_path.file_name().map_or(file_path, Path::new) } else { relative };
	relative.display().to_string()
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
				let languages = FileFilter { include: &[], exclude: &[], include_generated: true, attributes: None };
				let mut results = AnalysisResults::default();
				process_file(&path, &root, &mut results, true, languages, &mut DirCache::default()).unwrap();
				let from_disk = results.languages().next().map(|(lang, stats)| {
					let file = &stats.files_list()[0];
					(lang.name, *file.counts(), file.embedded().clone())
				});
				let from_bytes =
					count_bytes(&path, &std::fs::read(&path).unwrap(), languages, &mut DirCache::default())
						.map(|(lang, counts)| (lang.name, counts.lines, counts.embedded));
				assert_eq!(from_bytes, from_disk, "{}", path.display());
				checked += 1;
			}
		}
		assert!(checked > 100, "only {checked} fixtures");
	}
}
