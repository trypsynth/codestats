use memchr::memchr;

use super::{
	encoding::{self, FileEncoding, decode_bytes},
	line_classifier::{CommentState, LineType, classify_line, is_blank},
	stats::LineCounts,
};
use crate::langs::{Language, scoring::language_matchers};

/// Count the lines of a file's contents.
pub(super) fn count_lines(language: &Language, encoding: FileEncoding, bytes: &[u8]) -> LineCounts {
	if encoding::is_utf16(encoding.encoding) {
		return encoding::count_utf16(language, encoding, bytes);
	}
	let mut counts = LineCounts::default();
	let mut comment_state = CommentState::new();
	// Validating the whole file once is much cheaper than decoding it line by line.
	if let Ok(text) = std::str::from_utf8(bytes.get(encoding.bom_len..).unwrap_or_default()) {
		count_text(text, language, &mut counts, &mut comment_state);
	} else {
		// Invalid UTF-8 is replaced line by line, keeping one bad byte from affecting the rest of the file.
		for (i, line) in lines(bytes).enumerate() {
			let line = decode_bytes(&bytes[line], encoding, i == 0);
			counts.add(classify_line(&line, language, &mut comment_state, i == 0));
		}
	}
	counts
}

/// Count the lines of valid UTF-8 text, as classifying each with [`classify_line`] would.
///
/// Outside comments and strings, the lines before the next byte that could open one can only be code or blank, so they're swept without classifying each; only lines that might hold a marker go through the classifier.
fn count_text(text: &str, language: &Language, counts: &mut LineCounts, comment_state: &mut CommentState) {
	let bytes = text.as_bytes();
	let openers = &language_matchers(language).openers;
	// Markers that only count in the first column aren't among the openers, so those languages classify every line.
	let can_sweep = language.column_one_comments.is_empty();
	let mut pos = 0;
	while pos < bytes.len() {
		// The first line goes through the classifier, which checks it for a shebang.
		if can_sweep && pos > 0 && comment_state.is_clear() {
			let stop = openers.find(&bytes[pos..]).map_or(bytes.len(), |i| pos + i);
			while pos < stop {
				let end = line_end(bytes, pos);
				if end > stop {
					break;
				}
				counts.add(if is_blank(&bytes[pos..end]) { LineType::Blank } else { LineType::Code });
				pos = end;
			}
			if pos >= bytes.len() {
				break;
			}
		}
		let end = line_end(bytes, pos);
		// Lines end at a `\n`, which is never inside a multi-byte character, so each is valid UTF-8 on its own.
		counts.add(classify_line(&text[pos..end], language, comment_state, pos == 0));
		pos = end;
	}
}

/// Where the line starting at `start` ends, just past its `\n` or at the end of `bytes`.
#[inline]
fn line_end(bytes: &[u8], start: usize) -> usize {
	memchr(b'\n', &bytes[start..]).map_or(bytes.len(), |i| start + i + 1)
}

/// The byte ranges of `bytes`' lines, each including its `\n`.
fn lines(bytes: &[u8]) -> impl Iterator<Item = std::ops::Range<usize>> + '_ {
	let mut start = 0;
	std::iter::from_fn(move || {
		if start >= bytes.len() {
			return None;
		}
		let end = memchr(b'\n', &bytes[start..]).map_or(bytes.len(), |pos| start + pos + 1);
		let line = start..end;
		start = end;
		Some(line)
	})
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;

	#[rstest]
	#[case::empty("", &[])]
	#[case::single_no_newline("hello", &["hello"])]
	#[case::single_with_newline("hello\n", &["hello\n"])]
	#[case::multiple("line1\nline2\nline3", &["line1\n", "line2\n", "line3"])]
	#[case::crlf("line1\r\nline2\r\n", &["line1\r\n", "line2\r\n"])]
	#[case::blank_lines("\n\n", &["\n", "\n"])]
	fn test_lines(#[case] text: &str, #[case] expected: &[&str]) {
		let got: Vec<_> = lines(text.as_bytes()).map(|range| &text[range]).collect();
		assert_eq!(got, expected);
	}

	/// Sweeping must count exactly as classifying every line does, for every language, on random text built from its own markers.
	#[test]
	fn sweeping_matches_classifying_every_line() {
		let mut seed: u64 = 0x0BAD_5EED_1234_5678;
		let mut next = move |bound: usize| {
			seed ^= seed << 13;
			seed ^= seed >> 7;
			seed ^= seed << 17;
			usize::try_from(seed % bound as u64).unwrap()
		};
		for language in crate::langs::LANGUAGES {
			let mut pieces: Vec<&str> =
				vec!["a", "x = 1", " ", "\t", "\n", "\n", "\n", "\r\n", "  \n", "#!/bin/sh", ";", "é", "(", "\\"];
			pieces.extend(language.line_comments);
			pieces.extend(language.block_comments.iter().flat_map(|&(start, end, _)| [start, end]));
			pieces.extend(language.strings.iter().flat_map(|delimiter| [delimiter.open, delimiter.close]));
			pieces.extend(language.column_one_comments);
			pieces.extend(language.shebangs);
			for _ in 0..40 {
				let mut text = String::new();
				for _ in 0..next(60) {
					text.push_str(pieces[next(pieces.len())]);
				}
				let mut expected = LineCounts::default();
				let mut state = CommentState::new();
				for (i, line) in lines(text.as_bytes()).enumerate() {
					expected.add(classify_line(&text[line], language, &mut state, i == 0));
				}
				let mut got = LineCounts::default();
				count_text(&text, language, &mut got, &mut CommentState::new());
				assert_eq!(got, expected, "{}: {text:?}", language.name);
			}
		}
	}

	#[test]
	fn invalid_utf8_counts_like_valid_text() {
		let rust = crate::langs::LANGUAGES.iter().find(|l| l.name == "Rust").unwrap();
		let utf8 = FileEncoding { encoding: encoding_rs::UTF_8, bom_len: 0 };
		let valid = count_lines(rust, utf8, b"// note\nlet x = 1;\n\n");
		let invalid = count_lines(rust, utf8, b"// not\xFFe\nlet x = 1;\n\n");
		assert_eq!(valid, invalid);
		assert_eq!(valid, LineCounts { total: 3, code: 1, comment: 1, blank: 1, shebang: 0 });
	}

	#[test]
	fn bom_is_not_part_of_the_first_line() {
		let python = crate::langs::LANGUAGES.iter().find(|l| l.name == "Python").unwrap();
		let with_bom = FileEncoding { encoding: encoding_rs::UTF_8, bom_len: 3 };
		let counts = count_lines(python, with_bom, b"\xEF\xBB\xBF#!/usr/bin/env python\nx = 1\n");
		assert_eq!(counts.shebang, 1);
		assert_eq!(counts.code, 1);
	}
}
