pub(super) mod patterns;
pub mod scoring;

use std::borrow::Cow;

use self::patterns::get_candidates;
use super::data::{LANGUAGES, Language};

/// Score awarded for each comment style match when disambiguating languages.
const COMMENT_MATCH_SCORE: i32 = 50;
/// Score awarded for each keyword match when disambiguating languages.
const KEYWORD_MATCH_SCORE: i32 = 10;
/// Occurrences of a single keyword that count toward the score, so one identifier repeated hundreds of times (`mapping` in a C file, scored for LPC) can't outweigh several distinct keywords.
const MAX_KEYWORD_OCCURRENCES: usize = 3;

/// Calculate a language match score based on comment styles and keywords found in content.
///
/// The scoring algorithm works as follows:
/// - Each matching line/block comment pattern adds 50 points
/// - Each keyword occurrence adds 10 points
/// - Symbol-only languages (e.g., Brainfuck) require high symbol density when alphabetic content is present
///
/// This weighted scoring ensures comment patterns (strong indicators) outweigh
/// keywords (weaker indicators that may appear as identifiers in other languages).
#[inline]
fn score_language(lang: &Language, content: &str, tokens: &[&str]) -> i32 {
	if lang.line_comments.is_empty() && lang.block_comments.is_empty() && lang.keywords.is_empty() {
		return 0;
	}
	let mut score: i32 = 0;
	for comment in lang.line_comments {
		if has_line_comment(content, comment) {
			score = score.saturating_add(COMMENT_MATCH_SCORE);
		}
	}
	for &(start, end, _) in lang.block_comments {
		if content.contains(start) && content.contains(end) {
			score = score.saturating_add(COMMENT_MATCH_SCORE);
		}
	}
	let mut matched_chars: usize = 0;
	let mut keyword_hits: usize = 0;
	for keyword in lang.keywords {
		// If keyword contains special characters, use substring matching to handle cases like "@interface" in Objective-C, which wouldn't match via tokenization since @ is a delimiter.
		let count = if keyword.chars().any(|c| !c.is_ascii_alphanumeric() && c != '_') {
			let occurrences = count_substring_keyword(content, keyword);
			matched_chars = matched_chars.saturating_add(occurrences.saturating_mul(keyword.len()));
			occurrences
		} else {
			tokens.iter().filter(|token| token.eq_ignore_ascii_case(keyword)).count()
		};
		keyword_hits = keyword_hits.saturating_add(count);
		let capped = i32::try_from(count.min(MAX_KEYWORD_OCCURRENCES)).unwrap_or(0);
		score = score.saturating_add(capped * KEYWORD_MATCH_SCORE);
	}
	// Languages that share junk-drawer extensions (e.g. `.inc`) need real keyword evidence, not just a matching comment marker.
	if lang.require_keywords && keyword_hits == 0 {
		return 0;
	}
	// For symbol-only languages (all keywords are symbols), require high density if alphabetic content exists
	if is_symbol_only_language(lang) && !tokens.is_empty() {
		// Guard against false positives when a file has a few symbols but mostly alphabetic text.
		let non_whitespace = content.chars().filter(|c| !c.is_whitespace()).count();
		if non_whitespace > 0 {
			let matched_chars_u128 = matched_chars as u128;
			let non_whitespace_u128 = non_whitespace as u128;
			// Require at least 50% of non-whitespace chars to be language symbols
			if matched_chars_u128.saturating_mul(2) < non_whitespace_u128 {
				return 0;
			}
		}
	}
	score
}

/// Count occurrences of a punctuation-containing keyword.
///
/// A keyword ending in an identifier character must not be followed by one, so `@end` doesn't match inside Doxygen's `@endcode`.
fn count_substring_keyword(content: &str, keyword: &str) -> usize {
	let ends_in_word = keyword.bytes().last().is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_');
	if !ends_in_word {
		return content.matches(keyword).count();
	}
	content
		.match_indices(keyword)
		.filter(|(pos, _)| {
			content.as_bytes().get(pos + keyword.len()).is_none_or(|&b| !b.is_ascii_alphanumeric() && b != b'_')
		})
		.count()
}

/// Check whether any line starts with `marker` as a comment.
///
/// The marker must begin the line (after indentation) and must not be immediately followed by an identifier character, so that preprocessor directives like `#import` or format specifiers like `%ld` aren't mistaken for `#` or `%` comments.
fn has_line_comment(content: &str, marker: &str) -> bool {
	content.lines().any(|line| {
		line.trim_start()
			.strip_prefix(marker)
			.is_some_and(|rest| !rest.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_'))
	})
}

fn is_symbol_only_language(lang: &Language) -> bool {
	!lang.keywords.is_empty()
		&& lang.keywords.iter().all(|kw| kw.chars().all(|c| !c.is_ascii_alphanumeric() && c != '_'))
		&& lang.line_comments.is_empty()
		&& lang.block_comments.is_empty()
}

#[inline]
fn disambiguate<'a>(candidates: &[&'a Language], content: &str) -> Option<&'a Language> {
	let tokens: Vec<_> = tokenize(content).collect();
	// On ties, prefer the candidate with fewer keywords, then the earliest one. A language that extends another (e.g. Bison over Yacc) lists the base language's keywords plus its own, so without extension-specific keywords the base language is the better guess.
	let mut best: Option<(&'a Language, i32)> = None;
	for lang in candidates {
		let score = score_language(lang, content, &tokens);
		if score > 0
			&& best.is_none_or(|(best_lang, best_score)| {
				score > best_score || (score == best_score && lang.keywords.len() < best_lang.keywords.len())
			}) {
			best = Some((lang, score));
		}
	}
	best.map(|(lang, _)| lang)
}

#[inline]
fn tokenize(content: &str) -> impl Iterator<Item = &str> {
	content.split(|c: char| !c.is_ascii_alphanumeric() && c != '_').filter(|token| !token.is_empty())
}

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
fn detect_from_shebang(content: &str) -> Option<&'static Language> {
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

/// When nothing scores, fall back to the one candidate that doesn't require keyword evidence, if there is exactly one (e.g. C over Objective-C for a bare `.h`).
fn default_candidate(candidates: &[&'static Language]) -> Option<&'static Language> {
	let mut defaults = candidates.iter().filter(|lang| !lang.require_keywords);
	match (defaults.next(), defaults.next()) {
		(Some(lang), None) => Some(lang),
		_ => None,
	}
}

/// Whether a filename matches several languages, so its content decides which.
#[must_use]
pub fn is_ambiguous(filename: &str) -> bool {
	get_candidates(filename).len() > 1
}

#[must_use]
pub fn detect_language_info(filename: &str, content: Option<&str>) -> Option<&'static Language> {
	let candidates = get_candidates(filename);
	match candidates.len() {
		0 => content.and_then(detect_from_shebang),
		1 => Some(candidates[0]),
		_ => content
			.and_then(|file_content| {
				detect_from_shebang(file_content).or_else(|| disambiguate(&candidates, file_content))
			})
			.or_else(|| default_candidate(&candidates)),
	}
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;

	const TEST_LANGUAGE_ALPHA: Language = Language {
		index: 0,
		name: "Alpha",
		file_patterns: &["*.alpha"],
		line_comments: &["//"],
		block_comments: &[],
		column_one_comments: &[],
		shebangs: &[],
		keywords: &["alpha", "beta"],
		require_keywords: false,
		strings: &[],
	};

	const TEST_LANGUAGE_BETA: Language = Language {
		index: 1,
		name: "Beta",
		file_patterns: &["*.beta"],
		line_comments: &["#"],
		block_comments: &[],
		column_one_comments: &[],
		shebangs: &[],
		keywords: &["winner"],
		require_keywords: false,
		strings: &[],
	};

	#[test]
	fn score_language_combines_comments_and_keywords() {
		let content = "// comment\nalpha beta alpha";
		let tokens: Vec<_> = tokenize(content).collect();
		let score = score_language(&TEST_LANGUAGE_ALPHA, content, &tokens);
		assert_eq!(score, 50 + 3 * 10);
	}

	#[test]
	fn repeated_keyword_is_capped() {
		// One keyword repeated many times shouldn't beat several distinct keywords.
		const REPEATED: Language = Language { name: "Repeated", keywords: &["mapping"], ..TEST_LANGUAGE_ALPHA };
		const DISTINCT: Language =
			Language { name: "Distinct", keywords: &["void", "struct", "sizeof", "typedef"], ..TEST_LANGUAGE_ALPHA };
		let content = format!("// x\n{}void f(struct s); sizeof(int); typedef int t;", "mapping ".repeat(50));
		assert_eq!(disambiguate(&[&REPEATED, &DISTINCT], &content).unwrap().name, "Distinct");
	}

	#[test]
	fn disambiguate_prefers_highest_score() {
		let candidates = vec![&TEST_LANGUAGE_ALPHA, &TEST_LANGUAGE_BETA];
		let alpha_content = "alpha only";
		let beta_content = "# winner winner";
		let chosen_alpha = disambiguate(&candidates, alpha_content).unwrap();
		assert_eq!(chosen_alpha.name, "Alpha");
		let chosen_beta = disambiguate(&candidates, beta_content).unwrap();
		assert_eq!(chosen_beta.name, "Beta");
	}

	#[test]
	fn disambiguate_prefers_base_language_on_tie() {
		const BASE: Language = Language { name: "Base", keywords: &["alpha"], ..TEST_LANGUAGE_ALPHA };
		const EXTENDED: Language = Language { name: "Extended", keywords: &["alpha", "omega"], ..TEST_LANGUAGE_ALPHA };
		let candidates = vec![&EXTENDED, &BASE];
		assert_eq!(disambiguate(&candidates, "// alpha").unwrap().name, "Base");
		assert_eq!(disambiguate(&candidates, "// alpha omega").unwrap().name, "Extended");
	}

	#[test]
	fn detect_language_info_disambiguates_real_languages() {
		let content = "@interface Foo : NSObject\n@end\n";
		let language = detect_language_info("example.m", Some(content)).unwrap();
		assert_eq!(language.name, "Objective-C");
	}

	#[rstest]
	#[case::directives_and_format_specifiers(
		"example.m",
		"#import <Foundation/Foundation.h>\n#include <stdio.h>\n\nvoid f(NSInteger n) {\n\tNSLog(@\"%ld\", (long)n);\n}\n",
		"Objective-C"
	)]
	#[case::autoreleasepool(
		"main.m",
		"#import \"App.h\"\n\nint main(void) {\n\t@autoreleasepool {\n\t\t[[App alloc] init];\n\t}\n\treturn 0;\n}\n",
		"Objective-C"
	)]
	#[case::objc_header(
		"App.h",
		"#import <Foundation/Foundation.h>\n\n@interface App : NSObject\n- (void)run;\n@end\n",
		"Objective-C"
	)]
	#[case::c_header("util.h", "#ifndef UTIL_H\n#define UTIL_H\n\nint add(int a, int b);\n\n#endif\n", "C")]
	#[case::c_header_comments_only("util.h", "// helpers\n/* nothing here */\n", "C")]
	#[case::octave_function("example.m", "# compute\nfunction y = f(x)\n\ty = x;\nendfunction\n", "Octave")]
	fn detect_m_and_h_files(#[case] filename: &str, #[case] content: &str, #[case] expected: &str) {
		assert_eq!(detect_language_info(filename, Some(content)).map(|l| l.name), Some(expected));
	}

	#[rstest]
	#[case::whole_keyword("@end\n", "@end", 1)]
	#[case::doxygen_command("/** @endcode */", "@end", 0)]
	#[case::followed_by_punctuation("@end;", "@end", 1)]
	#[case::symbol_keyword("a -> b -> c", "->", 2)]
	fn test_count_substring_keyword(#[case] content: &str, #[case] keyword: &str, #[case] expected: usize) {
		assert_eq!(count_substring_keyword(content, keyword), expected);
	}

	#[rstest]
	#[case::hash_comment("# comment", "#", true)]
	#[case::indented("\t  # comment", "#", true)]
	#[case::bare_marker("#", "#", true)]
	#[case::directive("#import <Foundation/Foundation.h>", "#", false)]
	#[case::mid_line("printf(\"%ld\", x);", "%", false)]
	#[case::url("see https://example.com", "//", false)]
	fn test_has_line_comment(#[case] content: &str, #[case] marker: &str, #[case] expected: bool) {
		assert_eq!(has_line_comment(content, marker), expected);
	}

	#[rstest]
	#[case::bare_header("undef.h", "#undef STRINGLIB\n#undef STRINGLIB_LEN\n", "C")]
	#[case::bare_c_file("basic.c", "{\n\t\"empty\",\n\t.insns = { BPF_EXIT_INSN() },\n},\n", "C")]
	fn falls_back_to_default_candidate(#[case] filename: &str, #[case] content: &str, #[case] expected: &str) {
		assert_eq!(detect_language_info(filename, Some(content)).map(|l| l.name), Some(expected));
	}

	#[rstest]
	#[case::longer_extension("layout.blade.php", "Blade")]
	#[case::exact_filename("pom.xml", "Maven")]
	fn most_specific_pattern_wins(#[case] filename: &str, #[case] expected: &str) {
		assert_eq!(detect_language_info(filename, Some("")).map(|l| l.name), Some(expected));
	}

	#[test]
	fn repeated_identifier_does_not_make_c_lpc() {
		// CPython's C API talks about the mapping protocol constantly; that shouldn't make a C file LPC.
		let content = format!(
			"#include \"Python.h\"\n\nstatic PyObject *\nf(PyObject *self)\n{{\n{}    return NULL;\n}}\n",
			"    PyMapping_Check(mapping);\n".repeat(40)
		);
		assert_eq!(detect_language_info("dict.c", Some(&content)).map(|l| l.name), Some("C"));
	}

	#[test]
	fn inc_without_keywords_is_unrecognized() {
		// A kbd keymap: its `#` comments alone shouldn't make it BitBake.
		let content = "# Common keys\nkeycode 1 = Escape\nkeycode 2 = one exclam\n";
		assert!(detect_language_info("qwerty-layout.inc", Some(content)).is_none());
	}

	#[test]
	fn detect_language_info_skips_when_no_signal() {
		let language = detect_language_info("ambiguous.m", Some("plain text without hints"));
		assert!(language.is_none());
	}

	#[test]
	fn detect_brainfuck_with_ascii_comments() {
		let content =
			"This is a comment\n++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.\nMore comments here\n>+++.\n";
		let language = detect_language_info("example.bf", Some(content)).unwrap();
		assert_eq!(language.name, "Brainfuck");
	}

	#[test]
	fn detect_b_over_brainfuck() {
		let content = "/* B language */\nmain $(\nauto i;\ni = 0;\nwhile (i < 10) i++;$)";
		let language = detect_language_info("example.b", Some(content)).unwrap();
		assert_eq!(language.name, "B");
	}

	#[test]
	fn detect_b_with_many_comparison_operators() {
		let content = "main $(\n   auto ch;\n   if (ch > 0100 & ch < 0133) ch = ch + 040;\n   if (ch > 500 & ch < 600) goto loop;\n$)";
		let language = detect_language_info("example.b", Some(content)).unwrap();
		assert_eq!(language.name, "B");
	}

	#[rstest]
	#[case::splits_on_non_alnum("hello_world foo-bar", vec!["hello_world", "foo", "bar"])]
	#[case::empty("", vec![])]
	#[case::only_delimiters("!@#$%^&*()", vec![])]
	#[case::preserves_underscores("__init__ __main__", vec!["__init__", "__main__"])]
	fn test_tokenize(#[case] input: &str, #[case] expected: Vec<&str>) {
		let tokens: Vec<_> = tokenize(input).collect();
		assert_eq!(tokens, expected);
	}

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

	#[test]
	fn is_symbol_only_language_detects_brainfuck() {
		// Find Brainfuck in LANGUAGES
		let bf = LANGUAGES.iter().find(|l| l.name == "Brainfuck");
		assert!(bf.is_some());
		assert!(is_symbol_only_language(bf.unwrap()));
	}

	#[test]
	fn is_symbol_only_language_rejects_rust() {
		let rust = LANGUAGES.iter().find(|l| l.name == "Rust");
		assert!(rust.is_some());
		assert!(!is_symbol_only_language(rust.unwrap()));
	}

	#[test]
	fn single_candidate_returns_immediately() {
		// .rs files should have only Rust as candidate
		let lang = detect_language_info("test.rs", None);
		assert!(lang.is_some());
		assert_eq!(lang.unwrap().name, "Rust");
	}

	#[test]
	fn unknown_extension_with_shebang() {
		let content = "#!/usr/bin/env node\nconsole.log('hello');";
		let lang = detect_language_info("script", Some(content));
		assert!(lang.is_some());
		assert_eq!(lang.unwrap().name, "JavaScript");
	}

	#[test]
	fn unknown_extension_without_hints_returns_none() {
		let lang = detect_language_info("random_file", Some("just some text"));
		assert!(lang.is_none());
	}
}
