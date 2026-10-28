use std::{collections::HashMap, sync::LazyLock};

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

use super::super::data::{LANGUAGES, Language};

/// File patterns indexed for fast lookup. Nearly every pattern is an extension (`*.rs`) or an exact filename (`Makefile`), which are hash lookups; compiling all of them into one glob set made startup ~20ms slower.
pub(super) struct PatternIndex {
	/// Exact filenames, matched case-sensitively so `BUILD` doesn't claim a `build` script.
	filenames: HashMap<&'static str, Vec<usize>>,
	/// Lowercased extensions from `*.ext` patterns, matched case-insensitively. They may contain dots (`blade.php`).
	extensions: HashMap<String, Vec<usize>>,
	/// The few remaining globs (`Kconfig.*`), matched case-sensitively.
	globs: GlobSet,
	glob_langs: Vec<usize>,
	glob_specificity: Vec<usize>,
}

/// Rank a glob by specificity: longer literal text beats shorter. Exact filenames rank above every glob.
fn rank_glob(pattern: &str) -> usize {
	pattern.chars().filter(|c| !matches!(c, '*' | '?' | '[' | ']' | '{' | '}')).count()
}

fn is_glob(pattern: &str) -> bool {
	pattern.contains(['*', '?', '[', '{'])
}

pub(super) static PATTERN_INDEX: LazyLock<PatternIndex> = LazyLock::new(|| {
	let mut filenames: HashMap<&'static str, Vec<usize>> = HashMap::new();
	let mut extensions: HashMap<String, Vec<usize>> = HashMap::new();
	let mut builder = GlobSetBuilder::new();
	let mut glob_langs = Vec::new();
	let mut glob_specificity = Vec::new();
	for lang in LANGUAGES {
		for &pattern in lang.file_patterns {
			if !is_glob(pattern) {
				filenames.entry(pattern).or_default().push(lang.index);
			} else if let Some(ext) = pattern.strip_prefix("*.").filter(|ext| !is_glob(ext)) {
				extensions.entry(ext.to_ascii_lowercase()).or_default().push(lang.index);
			} else {
				let glob = GlobBuilder::new(pattern)
					.build()
					.unwrap_or_else(|e| panic!("Invalid glob pattern '{pattern}' for language {}: {e}", lang.name));
				builder.add(glob);
				glob_langs.push(lang.index);
				glob_specificity.push(rank_glob(pattern));
			}
		}
	}
	let globs = builder.build().expect("GlobSetBuilder should never fail with valid patterns from languages.json5");
	PatternIndex { filenames, extensions, globs, glob_langs, glob_specificity }
});

/// Collects the languages matched by the most specific patterns seen so far.
struct BestMatches {
	specificity: usize,
	langs: Vec<usize>,
}

impl BestMatches {
	fn consider(&mut self, specificity: usize, langs: &[usize]) {
		if specificity > self.specificity {
			self.specificity = specificity;
			self.langs.clear();
		}
		if specificity == self.specificity {
			self.langs.extend_from_slice(langs);
		}
	}
}

/// Find the candidate languages for a filename, keeping only those matched by the most specific pattern (`*.blade.php` over `*.php`, `pom.xml` over `*.xml`). Candidates are returned in language order.
#[inline]
pub fn get_candidates(filename: &str) -> Vec<&'static Language> {
	let index = &*PATTERN_INDEX;
	let mut best = BestMatches { specificity: 0, langs: Vec::new() };
	if let Some(langs) = index.filenames.get(filename) {
		best.consider(usize::MAX, langs);
	}
	let lower = filename.to_ascii_lowercase();
	for (dot, _) in lower.match_indices('.') {
		let ext = &lower[dot + 1..];
		if let Some(langs) = index.extensions.get(ext) {
			// Matches the glob ranking: `*.php` has four literal characters.
			best.consider(ext.len() + 1, langs);
		}
	}
	if !index.globs.is_empty() {
		for idx in index.globs.matches(filename) {
			best.consider(index.glob_specificity[idx], &[index.glob_langs[idx]]);
		}
	}
	best.langs.sort_unstable();
	best.langs.dedup();
	best.langs.into_iter().map(|idx| &LANGUAGES[idx]).collect()
}
