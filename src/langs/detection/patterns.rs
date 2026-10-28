use std::{cell::RefCell, sync::LazyLock};

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

use super::super::data::{LANGUAGES, Language};

pub(super) struct LanguageGlobs {
	pub(super) set: GlobSet,
	pub(super) pattern_lang_indexes: Vec<usize>,
	/// How specific each pattern is; only the most specific matches become candidates.
	pattern_specificity: Vec<usize>,
}

/// Rank a pattern by specificity: exact filenames beat globs, and longer literal text beats shorter (`*.blade.php` over `*.php`).
fn specificity(pattern: &str) -> usize {
	if is_glob(pattern) {
		pattern.chars().filter(|c| !matches!(c, '*' | '?' | '[' | ']' | '{' | '}')).count()
	} else {
		usize::MAX
	}
}

fn is_glob(pattern: &str) -> bool {
	pattern.contains(['*', '?', '[', '{'])
}

/// Extension patterns like `*.R` match case-insensitively. Exact filenames and other globs (`Kconfig.*`) match case-sensitively, so `BUILD` doesn't claim a `build` script.
fn is_extension_pattern(pattern: &str) -> bool {
	pattern.strip_prefix("*.").is_some_and(|ext| !is_glob(ext))
}

pub(super) static LANGUAGE_GLOBSET: LazyLock<LanguageGlobs> = LazyLock::new(|| {
	let mut builder = GlobSetBuilder::new();
	let mut pattern_lang_indexes = Vec::new();
	let mut pattern_specificity = Vec::new();
	for lang in LANGUAGES {
		for pattern in lang.file_patterns {
			let mut glob_builder = GlobBuilder::new(pattern);
			glob_builder.case_insensitive(is_extension_pattern(pattern));
			let glob = glob_builder
				.build()
				.unwrap_or_else(|e| panic!("Invalid glob pattern '{pattern}' for language {}: {e}", lang.name));
			// Record the language index so glob match indices can map back to LANGUAGES.
			pattern_lang_indexes.push(lang.index);
			pattern_specificity.push(specificity(pattern));
			builder.add(glob);
		}
	}
	let set = builder.build().expect("GlobSetBuilder should never fail with valid patterns from languages.json5");
	LanguageGlobs { set, pattern_lang_indexes, pattern_specificity }
});

// Thread-local buffer for tracking which languages have been seen during candidate gathering.
// Using thread-local storage avoids allocating a new Vec for every file processed, while remaining safe for parallel execution since each thread gets its own buffer.
thread_local! {
	static SEEN_BUFFER: RefCell<Vec<bool>> = RefCell::new(vec![false; LANGUAGES.len()]);
}

#[inline]
pub fn get_candidates(filename: &str) -> Vec<&'static Language> {
	let globs = &*LANGUAGE_GLOBSET;
	SEEN_BUFFER.with(|seen_cell| {
		let mut seen = seen_cell.borrow_mut();
		seen.fill(false);
		let mut candidates = Vec::new();
		let matches = globs.set.matches(filename);
		let best = matches.iter().map(|&idx| globs.pattern_specificity[idx]).max().unwrap_or(0);
		for match_idx in matches {
			if globs.pattern_specificity[match_idx] < best {
				continue;
			}
			let lang_idx = globs.pattern_lang_indexes[match_idx];
			if !seen[lang_idx] {
				seen[lang_idx] = true;
				candidates.push(&LANGUAGES[lang_idx]);
			}
		}
		candidates
	})
}
