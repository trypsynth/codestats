use std::borrow::Cow;

use memchr::memchr;

use super::{
	embedded::Classifier,
	stats::{FileCounts, LineCounts},
};
use crate::langs::Language;

/// Percentage of non-text bytes in a sample that indicates a binary file.
const BINARY_THRESHOLD_PERCENT: usize = 20;
/// How much decoded UTF-16 text to gather before counting its lines.
const UTF16_DECODE_CHUNK_SIZE: usize = 8 * 1024;

/// The encodings codestats reads: UTF-8, which also stands for anything else ASCII-compatible, or UTF-16.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Encoding {
	Utf8,
	Utf16Le,
	Utf16Be,
}

impl Encoding {
	/// The encoding a byte order mark at the start of `bytes` names, and the mark's length.
	const fn for_bom(bytes: &[u8]) -> Option<(Self, usize)> {
		match bytes {
			[0xEF, 0xBB, 0xBF, ..] => Some((Self::Utf8, 3)),
			[0xFF, 0xFE, ..] => Some((Self::Utf16Le, 2)),
			[0xFE, 0xFF, ..] => Some((Self::Utf16Be, 2)),
			_ => None,
		}
	}

	/// Decode `bytes`, replacing what isn't valid with U+FFFD. A byte order mark at the start is dropped, and decides the encoding.
	fn decode(self, bytes: &[u8]) -> Cow<'_, str> {
		let (encoding, bytes) = match Self::for_bom(bytes) {
			Some((encoding, len)) => (encoding, &bytes[len..]),
			None => (self, bytes),
		};
		match encoding {
			Self::Utf8 => String::from_utf8_lossy(bytes),
			Self::Utf16Le | Self::Utf16Be => {
				let mut text = String::with_capacity(bytes.len());
				encoding.decode_utf16_into(bytes, &mut text);
				Cow::Owned(text)
			}
		}
	}

	/// Append UTF-16 `bytes` to `output`, with U+FFFD for an unpaired surrogate or a trailing odd byte.
	fn decode_utf16_into(self, bytes: &[u8], output: &mut String) {
		let (pairs, odd) = bytes.as_chunks::<2>();
		let units = pairs.iter().map(|&pair| match self {
			Self::Utf16Be => u16::from_be_bytes(pair),
			_ => u16::from_le_bytes(pair),
		});
		output.extend(char::decode_utf16(units).map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER)));
		if !odd.is_empty() {
			output.push(char::REPLACEMENT_CHARACTER);
		}
	}
}

#[derive(Clone, Copy)]
pub(super) struct FileEncoding {
	pub(super) encoding: Encoding,
	pub(super) bom_len: usize,
}

pub(super) fn detect_encoding(samples: &[u8]) -> FileEncoding {
	if let Some((encoding, bom_len)) = Encoding::for_bom(samples) {
		FileEncoding { encoding, bom_len }
	} else {
		detect_utf16_without_bom(samples).unwrap_or(FileEncoding { encoding: Encoding::Utf8, bom_len: 0 })
	}
}

pub(super) fn decode_bytes(bytes: &[u8], encoding: FileEncoding, strip_bom: bool) -> Cow<'_, str> {
	let mut slice = bytes;
	if strip_bom && encoding.bom_len > 0 && slice.len() >= encoding.bom_len {
		slice = &slice[encoding.bom_len..];
	}
	encoding.encoding.decode(slice)
}

pub(super) fn is_probably_binary(sample: &[u8], encoding: FileEncoding) -> bool {
	if sample.is_empty() {
		return false;
	}
	if is_utf16(encoding.encoding) {
		return false;
	}
	if memchr(0, sample).is_some() {
		return true;
	}
	// Control characters other than tab, line feed and carriage return (NUL was ruled out above), plus DEL. Counted in byte-sized sums over chunks short enough not to overflow, which vectorizes many times wider than summing into a usize.
	let non_text: usize = sample
		.chunks(255)
		.map(|chunk| {
			let count = chunk.iter().fold(0u8, |count, &b| {
				count + u8::from(b < 0x20 && b != b'\t' && b != b'\n' && b != b'\r') + u8::from(b == 0x7F)
			});
			usize::from(count)
		})
		.sum();
	non_text * 100 / sample.len() > BINARY_THRESHOLD_PERCENT
}

pub(super) fn is_utf16(encoding: Encoding) -> bool {
	encoding != Encoding::Utf8
}

fn detect_utf16_without_bom(samples: &[u8]) -> Option<FileEncoding> {
	// UTF-16 text without a BOM is full of zero bytes, so a sample without any (nearly every text file) can't be it.
	if samples.len() < 4 || memchr(0, samples).is_none() {
		return None;
	}
	let (pairs, rest) = samples.as_chunks::<2>();
	let mut zero_even = usize::from(rest == [0]);
	let mut zero_odd = 0;
	for &[even, odd] in pairs {
		zero_even += usize::from(even == 0);
		zero_odd += usize::from(odd == 0);
	}
	// Heuristic: UTF-16 text typically has a strong zero-byte bias on one parity.
	// At least a quarter of the bytes, compared without rounding down so that one zero in a tiny file isn't enough.
	let total_zeros = zero_even + zero_odd;
	if total_zeros * 4 < samples.len() {
		return None;
	}
	let (encoding, text_byte) = if zero_odd >= zero_even.saturating_mul(2) {
		(Encoding::Utf16Le, 0)
	} else if zero_even >= zero_odd.saturating_mul(2) {
		(Encoding::Utf16Be, 1)
	} else {
		return None;
	};
	// Binary data such as a table of small little-endian integers has the same zero bias, so also require the byte paired with each zero to be text.
	let (mut zero_pairs, mut text_pairs) = (0usize, 0usize);
	for pair in samples.as_chunks::<2>().0 {
		if pair[1 - text_byte] == 0 {
			zero_pairs += 1;
			text_pairs += usize::from(matches!(pair[text_byte], b'\t' | b'\n' | b'\r' | 0x20..=0x7E));
		}
	}
	(text_pairs * 10 >= zero_pairs * 9).then_some(FileEncoding { encoding, bom_len: 0 })
}

/// Count the lines of a UTF-16 file, decoding it a chunk at a time.
pub(super) fn count_utf16(language: &Language, encoding: FileEncoding, bytes: &[u8]) -> FileCounts {
	let mut line_counts = LineCounts::default();
	let mut classifier = Classifier::new(language);
	let mut is_first_line = true;
	let mut pending = String::new();
	let text = bytes.get(encoding.bom_len..).unwrap_or_default();
	// Each chunk ends just past a line feed, so no code unit or surrogate pair is split between two.
	let mut rest = text;
	while !rest.is_empty() {
		let cut = chunk_end(rest, encoding.encoding);
		encoding.encoding.decode_utf16_into(&rest[..cut], &mut pending);
		rest = &rest[cut..];
		drain_lines(&mut pending, &mut line_counts, &mut classifier, &mut is_first_line, rest.is_empty());
	}
	FileCounts { lines: line_counts, embedded: classifier.into_embedded() }
}

/// Where to end the next chunk of UTF-16 `text`: just past a line feed after the first [`UTF16_DECODE_CHUNK_SIZE`] bytes, or at the end.
fn chunk_end(text: &[u8], encoding: Encoding) -> usize {
	if text.len() <= UTF16_DECODE_CHUNK_SIZE {
		return text.len();
	}
	let line_feed = if encoding == Encoding::Utf16Be { [0, b'\n'] } else { [b'\n', 0] };
	text[UTF16_DECODE_CHUNK_SIZE..]
		.as_chunks::<2>()
		.0
		.iter()
		.position(|&unit| unit == line_feed)
		.map_or(text.len(), |at| UTF16_DECODE_CHUNK_SIZE + at * 2 + 2)
}

fn drain_lines(
	pending: &mut String,
	line_counts: &mut LineCounts,
	classifier: &mut Classifier,
	is_first_line: &mut bool,
	flush_final: bool,
) {
	let mut start = 0usize;
	let bytes = pending.as_bytes();
	while let Some(pos) = memchr(b'\n', &bytes[start..]) {
		let line_end = start + pos + 1;
		let line = &pending[start..line_end];
		line_counts.add(classifier.classify(line, *is_first_line));
		*is_first_line = false;
		start = line_end;
	}
	if flush_final && start < pending.len() {
		let line = &pending[start..];
		line_counts.add(classifier.classify(line, *is_first_line));
		*is_first_line = false;
		start = pending.len();
	}
	if start > 0 {
		pending.drain(..start);
	} else if flush_final {
		pending.clear();
	}
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;

	#[rstest]
	#[case::non_utf8_text_is_not_flagged_as_binary(Encoding::Utf8, 0, vec![0xC3, 0x28, b'a', b'b'], false)]
	#[case::null_bytes_are_flagged_as_binary(Encoding::Utf8, 0, vec![0x00, b'a', b'b', b'c'], true)]
	#[case::empty_sample_is_not_binary(Encoding::Utf8, 0, vec![], false)]
	#[case::utf16_is_never_binary(Encoding::Utf16Le, 0, vec![0x00, 0x00, 0x00, 0x00], false)]
	#[case::high_control_chars_flagged_binary(Encoding::Utf8, 0, vec![0x01, 0x02, 0x03, b'a', b'b', b'c', b'd', b'e', b'f', b'g'], true)]
	fn test_is_probably_binary(
		#[case] encoding: Encoding,
		#[case] bom_len: usize,
		#[case] sample: Vec<u8>,
		#[case] expected: bool,
	) {
		let file_encoding = FileEncoding { encoding, bom_len };
		assert_eq!(is_probably_binary(&sample, file_encoding), expected);
	}

	#[rstest]
	#[case::detect_utf16_le_without_bom(vec![b'a', 0x00, b'b', 0x00, b'c', 0x00, b'd', 0x00], Encoding::Utf16Le, 0)]
	#[case::detect_utf16_be_without_bom(vec![0x00, b'a', 0x00, b'b', 0x00, b'c', 0x00, b'd'], Encoding::Utf16Be, 0)]
	#[case::detect_utf8_bom(vec![0xEF, 0xBB, 0xBF, b'h', b'e', b'l', b'l', b'o'], Encoding::Utf8, 3)]
	#[case::detect_utf16_le_bom(vec![0xFF, 0xFE, b'h', 0x00, b'e', 0x00], Encoding::Utf16Le, 2)]
	#[case::detect_utf16_be_bom(vec![0xFE, 0xFF, 0x00, b'h', 0x00, b'e'], Encoding::Utf16Be, 2)]
	#[case::detect_plain_utf8_default(b"hello world".to_vec(), Encoding::Utf8, 0)]
	fn test_detect_encoding(
		#[case] sample: Vec<u8>,
		#[case] expected_encoding: Encoding,
		#[case] expected_bom_len: usize,
	) {
		let result = detect_encoding(&sample);
		assert_eq!(result.encoding, expected_encoding);
		assert_eq!(result.bom_len, expected_bom_len);
	}

	#[rstest]
	#[case::utf16_le(Encoding::Utf16Le, true)]
	#[case::utf16_be(Encoding::Utf16Be, true)]
	#[case::utf8(Encoding::Utf8, false)]
	fn test_is_utf16(#[case] encoding: Encoding, #[case] expected: bool) {
		assert_eq!(is_utf16(encoding), expected);
	}

	#[rstest]
	#[case::le(Encoding::Utf16Le, &[0xFF, 0xFE], u16::to_le_bytes as fn(u16) -> [u8; 2])]
	#[case::be(Encoding::Utf16Be, &[0xFE, 0xFF], u16::to_be_bytes as fn(u16) -> [u8; 2])]
	fn count_utf16_across_chunks(#[case] encoding: Encoding, #[case] bom: &[u8], #[case] to_bytes: fn(u16) -> [u8; 2]) {
		// Big enough to span several decode chunks, ending in a line with no newline.
		let text = format!("{}x = 1", "x = 1\n// note\n\n".repeat(1000));
		let mut bytes = bom.to_vec();
		bytes.extend(text.encode_utf16().flat_map(to_bytes));
		assert!(bytes.len() > 3 * UTF16_DECODE_CHUNK_SIZE);
		let language = crate::langs::LANGUAGES.iter().find(|l| l.name == "Rust").unwrap();
		let file_encoding = FileEncoding { encoding, bom_len: bom.len() };
		let counts = count_utf16(language, file_encoding, &bytes).lines;
		assert_eq!(counts, LineCounts { total: 3001, code: 1001, comment: 1000, blank: 1000, shebang: 0 });
	}

	#[test]
	fn one_zero_in_a_tiny_file_is_not_utf16() {
		// A JSON test case with an unescaped NUL in a string.
		let sample = b"[\"a\0a\"]";
		let encoding = detect_encoding(sample);
		assert_eq!(encoding.encoding, Encoding::Utf8);
		assert!(is_probably_binary(sample, encoding));
	}

	#[rstest]
	#[case::odd_length(b"a\0b\0\n\0c".as_slice(), 2)]
	#[case::single_byte(b"a".as_slice(), 1)]
	// Seven bytes of output fill all but one byte of the first allocation, too little for the U+FFFD the odd byte becomes.
	#[case::nearly_full_output(b"[\"a\0a\"]".as_slice(), 1)]
	fn utf16_with_a_trailing_odd_byte_finishes(#[case] bytes: &[u8], #[case] lines: u64) {
		let language = crate::langs::LANGUAGES.iter().find(|l| l.name == "Rust").unwrap();
		let counts = count_utf16(language, FileEncoding { encoding: Encoding::Utf16Le, bom_len: 0 }, bytes).lines;
		assert_eq!(counts.total, lines);
	}

	#[test]
	fn little_endian_integers_are_binary_not_utf16() {
		// A table of u16/u32 values has a zero high byte in every pair, just like ASCII text in UTF-16LE.
		let sample: Vec<u8> = (0u16..2048).flat_map(|n| (n % 200).to_le_bytes()).collect();
		let encoding = detect_encoding(&sample);
		assert_eq!(encoding.encoding, Encoding::Utf8);
		assert!(is_probably_binary(&sample, encoding));
	}

	#[test]
	fn bomless_utf16_text_is_not_binary() {
		let sample: Vec<u8> =
			"fn main() {\n\tprintln!(\"hi\");\r\n}\n".repeat(50).encode_utf16().flat_map(u16::to_le_bytes).collect();
		let encoding = detect_encoding(&sample);
		assert_eq!(encoding.encoding, Encoding::Utf16Le);
		assert!(!is_probably_binary(&sample, encoding));
	}

	#[test]
	fn short_sample_no_utf16_detection() {
		// Less than 4 bytes can't detect UTF-16 without BOM
		let sample = [b'a', 0x00];
		let result = detect_utf16_without_bom(&sample);
		assert!(result.is_none());
	}

	#[test]
	fn decode_bytes_strips_bom() {
		let bytes = [0xEF, 0xBB, 0xBF, b'h', b'i'];
		let encoding = FileEncoding { encoding: Encoding::Utf8, bom_len: 3 };
		let decoded = decode_bytes(&bytes, encoding, true);
		assert_eq!(&*decoded, "hi");
	}

	#[test]
	fn decode_bytes_without_strip_includes_full_content() {
		let bytes = [0xEF, 0xBB, 0xBF, b'h', b'i'];
		let encoding = FileEncoding { encoding: Encoding::Utf8, bom_len: 3 };
		let decoded = decode_bytes(&bytes, encoding, false);
		// A byte order mark at the start of what's decoded is dropped all the same.
		assert_eq!(&*decoded, "hi");
	}
}
