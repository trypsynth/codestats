use std::sync::LazyLock;

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

use super::data::{LANGUAGES, Language};

pub(super) struct LanguageGlobs {
	pub(super) set: GlobSet,
	pub(super) pattern_lang_indexes: Vec<usize>,
}

pub(super) static LANGUAGE_GLOBSET: LazyLock<LanguageGlobs> = LazyLock::new(|| {
	let mut builder = GlobSetBuilder::new();
	let mut pattern_lang_indexes = Vec::new();
	for lang in LANGUAGES {
		for pattern in lang.file_patterns {
			let mut glob_builder = GlobBuilder::new(pattern);
			glob_builder.case_insensitive(true);
			let glob = glob_builder
				.build()
				.unwrap_or_else(|e| panic!("Invalid glob pattern '{pattern}' for language {}: {e}", lang.name));
			pattern_lang_indexes.push(lang.index);
			builder.add(glob);
		}
	}
	let set = builder.build().unwrap();
	LanguageGlobs { set, pattern_lang_indexes }
});

#[inline]
pub fn get_candidates(filename: &str) -> Vec<&'static Language> {
	let globs = &*LANGUAGE_GLOBSET;
	let mut seen = vec![false; LANGUAGES.len()];
	let mut candidates = Vec::new();
	for match_idx in globs.set.matches(filename) {
		let lang_idx = globs.pattern_lang_indexes[match_idx];
		if !seen[lang_idx] {
			seen[lang_idx] = true;
			candidates.push(&LANGUAGES[lang_idx]);
		}
	}
	candidates
}
