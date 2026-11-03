use memchr::memchr;

#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
use super::masks::BlockScanner;
use super::{
	embedded::{Classifier, RegionStarts, contains_pair, region_starts},
	encoding::{self, FileEncoding, decode_bytes},
	line_classifier::{LineType, is_blank, is_plain_line_comment},
	stats::{FileCounts, LineCounts},
};
use crate::langs::{
	Language,
	matchers::{ByteSet, language_matchers},
};

/// Count the lines of a file's contents.
pub(super) fn count_lines(language: &Language, encoding: FileEncoding, bytes: &[u8]) -> FileCounts {
	if encoding::is_utf16(encoding.encoding) {
		return encoding::count_utf16(language, encoding, bytes);
	}
	let mut counts = LineCounts::default();
	let mut classifier = Classifier::new(language);
	// Validating the whole file once is much cheaper than decoding it line by line, and SIMD validation is several times faster than the standard library's on text that isn't mostly ASCII.
	if let Ok(text) = simdutf8::basic::from_utf8(bytes.get(encoding.bom_len..).unwrap_or_default()) {
		count_text(text, language, &mut counts, &mut classifier);
	} else {
		// Invalid UTF-8 is replaced line by line, keeping one bad byte from affecting the rest of the file.
		for (i, line) in lines(bytes).enumerate() {
			let line = decode_bytes(&bytes[line], encoding, i == 0);
			counts.add(classifier.classify(&line, i == 0));
		}
	}
	FileCounts { lines: counts, embedded: classifier.into_embedded() }
}

/// Count the lines of valid UTF-8 text, as classifying each with `classifier` would.
///
/// Outside comments and strings, the lines before the next byte that could open one can only be code or blank, so they're swept without classifying each; only lines that might hold a marker go through the classifier.
fn count_text(text: &str, language: &Language, counts: &mut LineCounts, classifier: &mut Classifier) {
	let bytes = text.as_bytes();
	let matchers = language_matchers(language);
	let openers = &matchers.openers;
	// Neither markers that only count in the first column nor the starts of embedded regions are among the openers. Regions that start a line are checked for line by line; languages with other kinds classify every line.
	let (can_sweep, region_starts) = language.embedding.map_or((true, None), |embedding| {
		region_starts(embedding).map_or((false, None), |starts| (true, Some(starts)))
	});
	let can_sweep = can_sweep && language.column_one_comments.is_empty();
	// A line starting with a marker isn't always a comment where the preprocessor reads it (`# define` in assembly).
	let plain_comments = language.embedding.is_none() && !language.line_comments.is_empty() && !language.c_preprocessor;
	let quick = matchers
		.strings
		.as_ref()
		.filter(|_| language.embedding.is_none())
		.and_then(|strings| strings.quick.as_ref().map(|quick| (quick, strings)));
	// Regions are checked for line by line, which the masks don't help with.
	#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
	let mut scanner = region_starts.is_none().then(|| BlockScanner::new(bytes)).flatten();
	#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
	let openers_suit = BlockScanner::suits(openers.bytes());
	let mut next_opener = 0;
	let mut pos = 0;
	while pos < bytes.len() {
		// The first line goes through the classifier, which checks it for a shebang.
		if can_sweep && pos > 0 && classifier.is_clear() {
			loop {
				#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
				let at_stop = if let Some(scanner) = scanner.as_mut().filter(|_| openers_suit) {
					let (next, at_stop) = scanner.sweep(pos, openers.bytes(), LineType::Code, counts);
					pos = next;
					at_stop
				} else {
					sweep_by_search(bytes, openers, region_starts, &mut next_opener, &mut pos, counts)
				};
				#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
				let at_stop = sweep_by_search(bytes, openers, region_starts, &mut next_opener, &mut pos, counts);
				// A line stopped at only for its strings, which all close on it, is code like the others, and the sweep carries on after it.
				let end = line_end(bytes, pos);
				if at_stop {
					// A line stopped at that's just a line comment, which leaves nothing open, needn't go through the classifier either.
					if plain_comments && is_plain_line_comment(&text[pos..end], language) {
						counts.add(LineType::Comment);
						pos = end;
						continue;
					}
					if quick.is_some_and(|(quick, strings)| quick.is_plain_code(&bytes[pos..end], strings)) {
						counts.add(LineType::Code);
						pos = end;
						continue;
					}
				}
				break;
			}
			if pos >= bytes.len() {
				break;
			}
		}
		// Inside a comment or a string, lines holding none of the bytes that could end it stay inside it.
		#[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
		if can_sweep
			&& pos > 0
			&& !classifier.is_clear()
			&& let Some(scanner) = &mut scanner
			&& let Some((searched, with_text)) = classifier.what_ends_this(matchers)
			&& BlockScanner::suits(searched.bytes())
		{
			(pos, _) = scanner.sweep(pos, searched.bytes(), with_text, counts);
			if pos >= bytes.len() {
				break;
			}
		}
		let end = line_end(bytes, pos);
		// Lines end at a `\n`, which is never inside a multi-byte character, so each is valid UTF-8 on its own.
		counts.add(classifier.classify_before(&text[pos..end], pos == 0, Some(&text[end..])));
		pos = end;
	}
}

/// Count the lines from `pos` up to the first that might hold a marker or start a region, finding them by search. Returns whether it stopped at a marker, and leaves `pos` at the line it stopped at or the end of `bytes`.
fn sweep_by_search(
	bytes: &[u8],
	openers: &ByteSet,
	region_starts: Option<RegionStarts>,
	next_opener: &mut usize,
	pos: &mut usize,
	counts: &mut LineCounts,
) -> bool {
	// Stopping for a line that may start a region comes back here before reaching the opener. Without remembering it, a file of such lines with few openers takes time quadratic in its size.
	if *next_opener < *pos {
		*next_opener = openers.find(&bytes[*pos..]).map_or(bytes.len(), |i| *pos + i);
	}
	let stop = *next_opener;
	while *pos < stop {
		let end = line_end(bytes, *pos);
		if region_starts.is_some_and(|starts| may_start_region(&bytes[*pos..end], starts)) {
			return false;
		}
		if end > stop {
			return true;
		}
		counts.add(if is_blank(&bytes[*pos..end]) { LineType::Blank } else { LineType::Code });
		*pos = end;
	}
	// Any line left starts with the marker.
	*pos < bytes.len()
}

/// Whether `line` might start an embedded region: its first byte after whitespace is one that can, or isn't ASCII and so might be Unicode whitespace hiding one, which the classifier sorts out.
fn may_start_region(line: &[u8], starts: RegionStarts) -> bool {
	match starts {
		// The whitespace `str::trim_start` skips, as far as ASCII goes.
		RegionStarts::LineStart(starts) => line
			.iter()
			.find(|&&b| !matches!(b, b'\t'..=b'\r' | b' '))
			.is_some_and(|&b| !b.is_ascii() || starts.contains(&b)),
		RegionStarts::Anywhere(pair) => contains_pair(line, pair),
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
			let mut pieces: Vec<&str> = vec![
				"a",
				"x = 1",
				" ",
				"\t",
				"\n",
				"\n",
				"\n",
				"\r\n",
				"  \n",
				"#!/bin/sh",
				";",
				"é",
				"(",
				"\\",
				"<<EOF",
				"EOF",
				"<<<EOT",
				"EOT",
			];
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
				let mut classifier = Classifier::new(language);
				for (i, line) in lines(text.as_bytes()).enumerate() {
					expected.add(classifier.classify_before(&text[line.clone()], i == 0, Some(&text[line.end..])));
				}
				let mut got = LineCounts::default();
				count_text(&text, language, &mut got, &mut Classifier::new(language));
				assert_eq!(got, expected, "{}: {text:?}", language.name);
			}
		}
	}

	/// Preprocessor directives are code where `#` also starts comments, whichever way a line reaches the classifier.
	#[test]
	fn assembly_directives_stay_code_when_swept() {
		let assembly = crate::langs::language_named("Assembly").unwrap();
		let text = "mov r0, r1\n#define X 1\n# include <a.h>\n# a comment\nmov r1, r2\n";
		let mut counts = LineCounts::default();
		count_text(text, assembly, &mut counts, &mut Classifier::new(assembly));
		assert_eq!((counts.code, counts.comment), (4, 1));
	}

	/// A region's opening line must stop the sweep whatever whitespace it's indented with, as `str::trim_start` would skip it.
	#[test]
	fn sweeping_stops_at_indented_region_starts() {
		let openings = ["```rust", "~~~py", "[source,c]\n----", "#+BEGIN_SRC python", ".. code-block:: c"];
		for language in crate::langs::LANGUAGES.iter().filter(|language| language.embedding.is_some()) {
			for indent in ["", " \t", "\u{b}", "\u{c}", "\u{a0}"] {
				for opening in openings {
					let text = format!("x\ny\n{indent}{opening}\n  // c\n  # c\n----\n```\n#+end_src\nz\n");
					let mut expected = LineCounts::default();
					let mut classifier = Classifier::new(language);
					for (i, line) in lines(text.as_bytes()).enumerate() {
						expected.add(classifier.classify_before(&text[line.clone()], i == 0, Some(&text[line.end..])));
					}
					let mut got = LineCounts::default();
					count_text(&text, language, &mut got, &mut Classifier::new(language));
					assert_eq!(got, expected, "{}: {text:?}", language.name);
				}
			}
		}
	}

	#[test]
	fn invalid_utf8_counts_like_valid_text() {
		let rust = crate::langs::LANGUAGES.iter().find(|l| l.name == "Rust").unwrap();
		let utf8 = FileEncoding { encoding: encoding_rs::UTF_8, bom_len: 0 };
		let valid = count_lines(rust, utf8, b"// note\nlet x = 1;\n\n").lines;
		let invalid = count_lines(rust, utf8, b"// not\xFFe\nlet x = 1;\n\n").lines;
		assert_eq!(valid, invalid);
		assert_eq!(valid, LineCounts { total: 3, code: 1, comment: 1, blank: 1, shebang: 0 });
	}

	#[test]
	fn bom_is_not_part_of_the_first_line() {
		let python = crate::langs::LANGUAGES.iter().find(|l| l.name == "Python").unwrap();
		let with_bom = FileEncoding { encoding: encoding_rs::UTF_8, bom_len: 3 };
		let counts = count_lines(python, with_bom, b"\xEF\xBB\xBF#!/usr/bin/env python\nx = 1\n").lines;
		assert_eq!(counts.shebang, 1);
		assert_eq!(counts.code, 1);
	}
}
