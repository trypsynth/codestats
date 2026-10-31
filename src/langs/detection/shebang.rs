//! Recognizing a language from a file's `#!` line.

use std::borrow::Cow;

use crate::langs::data::{LANGUAGES, Language};

/// Normalize a shebang line: drop an optional space after `#!` and env's `-S` flag (`#!/usr/bin/env -S python3 -u`).
#[inline]
fn normalize_shebang(line: &str) -> Cow<'_, str> {
	let mut normalized = line.strip_prefix("#! ").map_or(Cow::Borrowed(line), |rest| Cow::Owned(format!("#!{rest}")));
	if let Some(idx) = normalized.find("/env -S ") {
		normalized = Cow::Owned(format!("{}/env {}", &normalized[..idx], &normalized[idx + "/env -S ".len()..]));
	}
	normalized
}

/// Length of the longest of `lang`'s shebangs that a normalized shebang line starts with. The interpreter name must end there, apart from a version (`python3.12`), so `node` doesn't claim `nodemon`.
#[inline]
fn shebang_match_len(normalized: &str, lang: &Language) -> Option<usize> {
	lang.shebangs
		.iter()
		.filter(|shebang| {
			normalized.strip_prefix(**shebang).is_some_and(|rest| !rest.starts_with(|c: char| c.is_ascii_alphabetic()))
		})
		.map(|shebang| shebang.len())
		.max()
}

/// Whether a (trimmed) first line is a shebang for `lang`.
#[must_use]
#[inline]
pub fn is_shebang_for(line: &str, lang: &Language) -> bool {
	!lang.shebangs.is_empty() && line.starts_with("#!") && shebang_match_len(&normalize_shebang(line), lang).is_some()
}

/// Detect a language from a shebang, preferring the longest match (`perl6` is Raku, not Perl).
#[inline]
pub(super) fn detect_from_shebang(content: &str) -> Option<&'static Language> {
	let trimmed = content.lines().next()?.trim();
	if !trimmed.starts_with("#!") {
		return None;
	}
	let normalized = normalize_shebang(trimmed);
	// Iterating in reverse makes `max_by_key` keep the earliest language on a tie.
	LANGUAGES
		.iter()
		.rev()
		.filter_map(|lang| shebang_match_len(&normalized, lang).map(|len| (len, lang)))
		.max_by_key(|&(len, _)| len)
		.map(|(_, lang)| lang)
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;

	#[rstest]
	#[case::removes_space("#! /usr/bin/env python", "#!/usr/bin/env python")]
	#[case::leaves_compact_unchanged("#!/bin/bash", "#!/bin/bash")]
	#[case::removes_env_split_flag("#!/usr/bin/env -S python3 -u", "#!/usr/bin/env python3 -u")]
	fn test_normalize_shebang(#[case] input: &str, #[case] expected: &str) {
		assert_eq!(&*normalize_shebang(input), expected);
	}

	#[rstest]
	#[case::python("#!/usr/bin/env python3\nprint('hello')", Some("Python"))]
	#[case::bash("#!/bin/bash\necho hello", Some("Bash"))]
	#[case::env_split("#!/usr/bin/env -S deno run --allow-net\nconsole.log(1)", Some("TypeScript"))]
	#[case::versioned("#!/usr/bin/env python3.12\nprint(1)", Some("Python"))]
	#[case::longest_match("#!/usr/bin/env perl6\nsay 1", Some("Raku"))]
	#[case::interpreter_must_end("#!/usr/bin/env nodemon\nx", None)]
	#[case::no_shebang("print('hello')\n# not a shebang", None)]
	#[case::empty("", None)]
	fn test_detect_from_shebang(#[case] content: &str, #[case] expected_name: Option<&str>) {
		let result = detect_from_shebang(content);
		assert_eq!(result.map(|l| l.name), expected_name);
	}
}
