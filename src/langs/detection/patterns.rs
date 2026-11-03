use super::super::data::{EXTENSIONS, FILENAMES, LANGUAGES, Language, PREFIXES};

/// Collects the languages matched by the most specific patterns seen so far.
struct BestMatches {
	specificity: usize,
	langs: Vec<u16>,
}

impl BestMatches {
	fn consider(&mut self, specificity: usize, langs: &[u16]) {
		if specificity > self.specificity {
			self.specificity = specificity;
			self.langs.clear();
		}
		if specificity == self.specificity {
			self.langs.extend_from_slice(langs);
		}
	}
}

/// Find the candidate languages for a filename, keeping only those matched by the most specific pattern (`*.blade.php` over `*.php`, `pom.xml` over `*.xml`), where a pattern is as specific as it has literal characters and an exact filename beats the rest. Exact filenames and prefixes (`Kconfig.*`) match case-sensitively, so `BUILD` doesn't claim a `build` script, and extensions don't. Candidates are returned in language order.
#[inline]
pub fn get_candidates(filename: &str) -> Vec<&'static Language> {
	let mut best = BestMatches { specificity: 0, langs: Vec::new() };
	if let Some(langs) = FILENAMES.get(filename) {
		best.consider(usize::MAX, langs);
	}
	let lower = filename.to_ascii_lowercase();
	for (dot, _) in lower.match_indices('.') {
		let extension = &lower[dot + 1..];
		if let Some(langs) = EXTENSIONS.get(extension) {
			// `*.php` has four literal characters.
			best.consider(extension.len() + 1, langs);
		}
	}
	for &(prefix, lang) in PREFIXES {
		if filename.starts_with(prefix) {
			best.consider(prefix.len(), &[lang]);
		}
	}
	best.langs.sort_unstable();
	best.langs.dedup();
	best.langs.into_iter().map(|index| &LANGUAGES[usize::from(index)]).collect()
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Nothing in a name built from a pattern is more specific than that pattern, so every pattern must find its language.
	#[test]
	fn every_pattern_finds_its_language() {
		for language in LANGUAGES {
			for pattern in language.file_patterns {
				let name =
					pattern.strip_suffix('*').map_or_else(|| (*pattern).to_string(), |prefix| format!("{prefix}x"));
				let name = name.strip_prefix('*').map_or_else(|| name.clone(), |rest| format!("x{rest}"));
				assert!(
					get_candidates(&name).iter().any(|candidate| candidate.name == language.name),
					"`{pattern}` doesn't find {}",
					language.name
				);
			}
		}
		assert_eq!(get_candidates("no-such-language.zzzzqq"), Vec::<&Language>::new());
		assert_eq!(get_candidates(""), Vec::<&Language>::new());
	}

	#[test]
	fn the_most_specific_pattern_wins() {
		let names = |file: &str| get_candidates(file).iter().map(|language| language.name).collect::<Vec<_>>();
		assert_eq!(names("page.blade.php"), ["Blade"]);
		assert_eq!(names("Kconfig.debug"), ["Kconfig"]);
		assert_eq!(names("Dockerfile.prod"), ["Dockerfile"]);
		assert_eq!(names("Dockerfile"), ["Dockerfile"]);
		// A longer extension outranks a prefix of fewer characters, but not the other way around.
		assert!(names("page.PHP").contains(&"PHP"));
	}
}
