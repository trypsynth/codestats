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

/// The interpreter a shebang runs, by name: the last part of its path, or the program `env` runs (`perl` in `#!./perl`, `#!/usr/local/bin/perl -w`, and `#!/usr/bin/env perl`).
fn interpreter(normalized: &str) -> Option<&str> {
	let mut words = normalized.strip_prefix("#!")?.split_ascii_whitespace();
	let first = words.next()?;
	let program = first.rsplit('/').next()?;
	if program != "env" {
		return Some(program);
	}
	words.find(|word| !word.starts_with('-') && !word.contains('='))
}

/// Length of the longest interpreter name of `lang`'s shebangs that `name` is, apart from a version, for a shebang no shebang of any language starts (`#!/usr/local/bin/perl`).
fn interpreter_match_len(name: &str, lang: &Language) -> Option<usize> {
	lang.shebangs
		.iter()
		.filter_map(|shebang| interpreter(shebang))
		.filter(|known| {
			name.strip_prefix(known).is_some_and(|rest| !rest.starts_with(|c: char| c.is_ascii_alphabetic()))
		})
		.map(str::len)
		.max()
}

/// Whether a (trimmed) first line is a shebang for `lang`.
#[must_use]
#[inline]
pub fn is_shebang_for(line: &str, lang: &Language) -> bool {
	if lang.shebangs.is_empty() || !line.starts_with("#!") {
		return false;
	}
	let normalized = normalize_shebang(line);
	shebang_match_len(&normalized, lang).is_some()
		|| interpreter(&normalized).is_some_and(|name| interpreter_match_len(name, lang).is_some())
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
	let longest = |matched: &dyn Fn(&Language) -> Option<usize>| {
		LANGUAGES
			.iter()
			.rev()
			.filter_map(|lang| matched(lang).map(|len| (len, lang)))
			.max_by_key(|&(len, _)| len)
			.map(|(_, lang)| lang)
	};
	let language = longest(&|lang| shebang_match_len(&normalized, lang)).or_else(|| {
		let name = interpreter(&normalized)?;
		longest(&|lang| interpreter_match_len(name, lang))
	})?;
	Some(restarted_as(content, language).unwrap_or(language))
}

/// The language a shell script restarts itself in, as Tcl scripts written for systems without `/usr/bin/env` do, under a comment the shell ends at a `\` but Tcl doesn't: `exec tclsh "$0" "$@"` in one of its first lines.
fn restarted_as(content: &str, language: &'static Language) -> Option<&'static Language> {
	if language.name != "Bash" {
		return None;
	}
	let restarts = content.lines().take(5).any(|line| {
		let line = line.trim_start();
		["exec tclsh", "exec wish", "exec jimsh"].iter().any(|exec| line.starts_with(exec))
	});
	restarts.then(|| crate::langs::language_named("Tcl")).flatten()
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
	#[case::tcl_restart(
		"#!/bin/sh\n# the next line restarts using tclsh \\\nexec tclsh \"$0\" \"$@\"\nputs hi",
		Some("Tcl")
	)]
	#[case::relative_interpreter("#!./perl\nprint 1;", Some("Perl"))]
	#[case::bare_interpreter("#!perl -w\nprint 1;", Some("Perl"))]
	#[case::other_prefix("#!/usr/local/bin/ruby\nputs 1", Some("Ruby"))]
	#[case::other_prefix_versioned("#!/opt/python3.12/bin/python3.12\nprint(1)", Some("Python"))]
	#[case::env_with_assignment("#!/usr/bin/env LC_ALL=C perl\nprint 1;", Some("Perl"))]
	#[case::named_interpreter_must_end("#!/usr/local/bin/nodemon\nx", None)]
	#[case::tk("#!/usr/bin/env wish\nbutton .b", Some("Tcl"))]
	#[case::rakudo("#!/usr/bin/env rakudo\nsay 1", Some("Raku"))]
	#[case::shell_running_tclsh_later("#!/bin/sh\nset -e\necho a\necho b\necho c\nexec tclsh build.tcl", Some("Bash"))]
	fn test_detect_from_shebang(#[case] content: &str, #[case] expected_name: Option<&str>) {
		let result = detect_from_shebang(content);
		assert_eq!(result.map(|l| l.name), expected_name);
	}
}
