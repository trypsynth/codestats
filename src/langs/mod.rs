mod data;
mod detection;
mod globset;
pub mod matchers;
pub mod printer;

pub use data::{LANGUAGES, Language};
pub use detection::detect_language_info;
pub use globset::language_globset;

#[cfg(test)]
mod tests {
	use super::{globset::get_candidates, *};

	#[test]
	fn get_candidates_uses_literal_map() {
		let candidates = get_candidates("Makefile");
		assert_eq!(candidates.len(), 1);
		assert_eq!(candidates[0].name, "Makefile");
	}

	#[test]
	fn get_candidates_handles_case_insensitive_literals() {
		let candidates = get_candidates("MAKEFILE");
		assert_eq!(candidates.len(), 1);
		assert_eq!(candidates[0].name, "Makefile");
	}

	#[test]
	fn get_candidates_supports_wildcards() {
		let candidates = get_candidates("lib.rs");
		assert!(candidates.iter().any(|lang| lang.name == "Rust"));
	}

	#[test]
	fn globset_matches_common_extensions() {
		assert!(language_globset().is_match("main.rs"));
		assert!(language_globset().is_match("README.md"));
	}
}
