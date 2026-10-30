use memchr::memchr;

use super::{
	encoding::{self, FileEncoding, decode_bytes},
	line_classifier::{CommentState, classify_line},
	stats::LineCounts,
};
use crate::langs::Language;

/// Count the lines of a file's contents.
pub(super) fn count_lines(language: &Language, encoding: FileEncoding, bytes: &[u8]) -> LineCounts {
	if encoding::is_utf16(encoding.encoding) {
		return encoding::count_utf16(language, encoding, bytes);
	}
	let mut counts = LineCounts::default();
	let mut comment_state = CommentState::new();
	let mut classify = |line: &str, is_first_line| {
		counts.add(classify_line(line, language, &mut comment_state, is_first_line));
	};
	// Validating the whole file once is much cheaper than decoding it line by line.
	if let Ok(text) = std::str::from_utf8(bytes.get(encoding.bom_len..).unwrap_or_default()) {
		for (i, line) in lines(text.as_bytes()).enumerate() {
			// Lines end at a `\n`, which is never inside a multi-byte character, so each is valid UTF-8 on its own.
			classify(&text[line], i == 0);
		}
	} else {
		// Invalid UTF-8 is replaced line by line, keeping one bad byte from affecting the rest of the file.
		for (i, line) in lines(bytes).enumerate() {
			classify(&decode_bytes(&bytes[line], encoding, i == 0), i == 0);
		}
	}
	counts
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
