pub(super) mod patterns;
mod scoring;
mod shebang;

pub use self::{patterns::get_candidates, shebang::is_shebang_for};
use self::{
	scoring::{KEYWORD_MATCH_SCORE, disambiguate},
	shebang::detect_from_shebang,
};
use super::data::Language;

/// When nothing scores, fall back to the one candidate that doesn't require keyword evidence, if there is exactly one (e.g. C over Objective-C for a bare `.h`).
fn default_candidate(candidates: &[&'static Language]) -> Option<&'static Language> {
	let mut defaults = candidates.iter().filter(|lang| !lang.require_keywords);
	match (defaults.next(), defaults.next()) {
		(Some(lang), None) => Some(lang),
		_ => None,
	}
}

/// The language detected for a file, and whether content decided it by a narrow margin for an unexpected language, so that more of the file might change the answer.
#[derive(Debug, Clone, Copy)]
pub struct Detection {
	pub language: Option<&'static Language>,
	pub close_call: bool,
}

/// How far the winning candidate must beat the runner-up (three keyword hits) for a decision from a small sample to stand.
const DECISIVE_MARGIN: i32 = 3 * KEYWORD_MATCH_SCORE;

/// Pick a language from a filename's candidates (see [`get_candidates`]). Content is only consulted when there isn't exactly one candidate.
#[must_use]
pub fn detect_language(candidates: &[&'static Language], content: Option<&str>) -> Detection {
	let decided = |language| Detection { language, close_call: false };
	match candidates.len() {
		0 => decided(content.and_then(detect_from_shebang)),
		// A shebang claims a file only where the extension's language says it may (BRLTTY's `latex-access.ctb` is Python); elsewhere a shebang atop, say, a `.cxx` lexer test or a dpatch `.patch` doesn't change what the file is.
		1 if candidates[0].shebang_overrides => decided(content.and_then(detect_from_shebang).or(Some(candidates[0]))),
		1 => decided(Some(candidates[0])),
		_ => {
			let Some(file_content) = content else {
				return decided(default_candidate(candidates));
			};
			if let Some(language) = detect_from_shebang(file_content) {
				return decided(Some(language));
			}
			let (language, margin) = disambiguate(candidates, file_content);
			let default = default_candidate(candidates);
			// Only a narrow win for something other than the default (LPC or Objective-C over C) is worth a closer look; a narrow win for the default is the expected answer.
			let surprising = language.is_none() || language.map(|l| l.index) != default.map(|l| l.index);
			Detection { language: language.or(default), close_call: margin < DECISIVE_MARGIN && surprising }
		}
	}
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;

	fn detect_language_info(filename: &str, content: Option<&str>) -> Option<&'static Language> {
		detect_language(&get_candidates(filename), content).language
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
	#[case::cpp_header(
		"widget.h",
		"#ifndef WIDGET_H\n#define WIDGET_H\n\nnamespace ui {\nclass Widget : public Base {\npublic:\n\tvirtual ~Widget();\n};\n}\n\n#endif\n",
		"C++"
	)]
	#[case::c_header_mentioning_cpp_in_comments(
		"mm.h",
		"/* Maps virtual memory into the caller's namespace; see the template above. */\n#ifndef MM_H\n#define MM_H\nstruct page;\nvoid map(struct page *p);\n#endif\n",
		"C"
	)]
	#[case::uppercase_is_not_a_cpp_keyword(
		"cfg.h",
		"#define VIRTUAL 1\n#define TEMPLATE 2\ntypedef int OPERATOR;\n",
		"C"
	)]
	#[case::bare_header_defaults_to_c("empty.h", "#pragma once\n", "C")]
	#[case::executable_contraction_table("latex-access.ctb", "#!/usr/bin/python3\nimport sys\n", "Python")]
	#[case::shebang_atop_a_lexer_test("x.cxx", "#!/usr/bin/env node\nint main() {}\n", "C++")]
	#[case::dpatch("01-fix.patch", "#! /bin/sh /usr/share/dpatch/dpatch-run\n--- a/x\n+++ b/x\n", "Diff")]
	#[case::c_header_comments_only("util.h", "// helpers\n/* nothing here */\n", "C")]
	#[case::octave_function("example.m", "# compute\nfunction y = f(x)\n\ty = x;\nendfunction\n", "Octave")]
	fn detect_m_and_h_files(#[case] filename: &str, #[case] content: &str, #[case] expected: &str) {
		assert_eq!(detect_language_info(filename, Some(content)).map(|l| l.name), Some(expected));
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
