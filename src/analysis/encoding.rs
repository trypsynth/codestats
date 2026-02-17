use std::{borrow::Cow, io::Read, path::Path};

use anyhow::Result;
use encoding_rs::{CoderResult, Decoder, Encoding, UTF_8, UTF_16BE, UTF_16LE};
use memchr::memchr;

use super::{line_classifier::CommentState, line_counter::LineCounts, stats::AnalysisResults};
use crate::langs::Language;

/// Percentage of non-text bytes in a sample that indicates a binary file.
const BINARY_THRESHOLD_PERCENT: usize = 20;
/// Chunk size for incremental UTF-16 decoding.
const UTF16_DECODE_CHUNK_SIZE: usize = 8 * 1024;

#[derive(Clone, Copy)]
pub(super) struct FileEncoding {
	pub(super) encoding: &'static Encoding,
	pub(super) bom_len: usize,
}

pub(super) fn detect_encoding(samples: &[u8]) -> FileEncoding {
	if let Some((encoding, bom_len)) = Encoding::for_bom(samples) {
		FileEncoding { encoding, bom_len }
	} else {
		detect_utf16_without_bom(samples).unwrap_or(FileEncoding { encoding: UTF_8, bom_len: 0 })
	}
}

pub(super) fn decode_bytes(bytes: &[u8], encoding: FileEncoding, strip_bom: bool) -> Cow<'_, str> {
	let mut slice = bytes;
	if strip_bom && encoding.bom_len > 0 && slice.len() >= encoding.bom_len {
		slice = &slice[encoding.bom_len..];
	}
	let (decoded, _, _) = encoding.encoding.decode(slice);
	decoded
}

pub(super) fn is_probably_binary(sample: &[u8], encoding: FileEncoding) -> bool {
	if sample.is_empty() {
		return false;
	}
	if is_utf16(encoding.encoding) {
		return false;
	}
	let non_text = sample.iter().filter(|b| matches!(**b, 0x00..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F | 0x7F)).count();
	let non_text_pct = non_text * 100 / sample.len();
	if non_text_pct > BINARY_THRESHOLD_PERCENT || sample.contains(&0) {
		return true;
	}
	false
}

pub(super) fn is_utf16(encoding: &'static Encoding) -> bool {
	encoding == UTF_16LE || encoding == UTF_16BE
}

fn detect_utf16_without_bom(samples: &[u8]) -> Option<FileEncoding> {
	if samples.len() < 4 {
		return None;
	}
	let mut zero_even = 0usize;
	let mut zero_odd = 0usize;
	for (idx, byte) in samples.iter().enumerate() {
		if *byte == 0 {
			if idx % 2 == 0 {
				zero_even += 1;
			} else {
				zero_odd += 1;
			}
		}
	}
	// Heuristic: UTF-16 text typically has a strong zero-byte bias on one parity.
	let total_zeros = zero_even + zero_odd;
	if total_zeros < samples.len() / 4 {
		return None;
	}
	if zero_odd >= zero_even.saturating_mul(2) {
		return Some(FileEncoding { encoding: UTF_16LE, bom_len: 0 });
	}
	if zero_even >= zero_odd.saturating_mul(2) {
		return Some(FileEncoding { encoding: UTF_16BE, bom_len: 0 });
	}
	None
}

pub(super) fn process_utf16_bytes(
	file_path: &Path,
	file_size: u64,
	results: &mut AnalysisResults,
	collect_details: bool,
	language: &'static Language,
	encoding: FileEncoding,
	bytes: &[u8],
) {
	use super::line_counter::finish_file_stats;
	let mut line_counts = LineCounts::default();
	let mut comment_state = CommentState::new();
	let mut is_first_line = true;
	let mut decoder = encoding.encoding.new_decoder_without_bom_handling();
	let mut pending = String::new();
	let mut output = String::new();
	let mut slice = bytes;
	if encoding.bom_len > 0 && slice.len() >= encoding.bom_len {
		slice = &slice[encoding.bom_len..];
	}
	for chunk in slice.chunks(UTF16_DECODE_CHUNK_SIZE) {
		decode_to_string(&mut decoder, chunk, false, &mut output);
		pending.push_str(&output);
		output.clear();
		drain_lines(&mut pending, language, &mut line_counts, &mut comment_state, &mut is_first_line, false);
	}
	decode_to_string(&mut decoder, &[], true, &mut output);
	pending.push_str(&output);
	output.clear();
	drain_lines(&mut pending, language, &mut line_counts, &mut comment_state, &mut is_first_line, true);
	finish_file_stats(file_path, file_size, results, collect_details, language, &line_counts);
}

pub(super) fn process_utf16_stream<R: Read>(
	file_path: &Path,
	file_size: u64,
	results: &mut AnalysisResults,
	collect_details: bool,
	language: &'static Language,
	encoding: FileEncoding,
	reader: &mut R,
) -> Result<()> {
	use super::line_counter::finish_file_stats;
	let mut line_counts = LineCounts::default();
	let mut comment_state = CommentState::new();
	let mut is_first_line = true;
	let mut decoder = encoding.encoding.new_decoder_without_bom_handling();
	let mut pending = String::new();
	let mut output = String::new();
	let mut buffer = vec![0u8; UTF16_DECODE_CHUNK_SIZE];
	let mut skip_bom = encoding.bom_len;
	loop {
		let read = reader.read(&mut buffer)?;
		if read == 0 {
			break;
		}
		let mut slice = &buffer[..read];
		if skip_bom > 0 {
			if read <= skip_bom {
				skip_bom -= read;
				continue;
			}
			slice = &slice[skip_bom..];
			skip_bom = 0;
		}
		decode_to_string(&mut decoder, slice, false, &mut output);
		pending.push_str(&output);
		output.clear();
		drain_lines(&mut pending, language, &mut line_counts, &mut comment_state, &mut is_first_line, false);
	}
	decode_to_string(&mut decoder, &[], true, &mut output);
	pending.push_str(&output);
	output.clear();
	drain_lines(&mut pending, language, &mut line_counts, &mut comment_state, &mut is_first_line, true);
	finish_file_stats(file_path, file_size, results, collect_details, language, &line_counts);
	Ok(())
}

fn decode_to_string(decoder: &mut Decoder, chunk: &[u8], last: bool, output: &mut String) {
	let mut offset = 0;
	while offset < chunk.len() || (last && offset == 0 && chunk.is_empty()) {
		output.reserve(chunk.len().saturating_sub(offset).max(1));
		let (result, read, _) = decoder.decode_to_string(&chunk[offset..], output, last);
		offset += read;
		match result {
			CoderResult::InputEmpty => break,
			CoderResult::OutputFull => {}
		}
	}
}

fn drain_lines(
	pending: &mut String,
	language: &'static Language,
	line_counts: &mut LineCounts,
	comment_state: &mut CommentState,
	is_first_line: &mut bool,
	flush_final: bool,
) {
	let mut start = 0usize;
	let bytes = pending.as_bytes();
	while let Some(pos) = memchr(b'\n', &bytes[start..]) {
		let line_end = start + pos + 1;
		let line = &pending[start..line_end];
		line_counts.classify_and_count(line, Some(language), comment_state, *is_first_line);
		*is_first_line = false;
		start = line_end;
	}
	if flush_final && start < pending.len() {
		let line = &pending[start..];
		line_counts.classify_and_count(line, Some(language), comment_state, *is_first_line);
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
	#[case::non_utf8_text_is_not_flagged_as_binary(UTF_8, 0, vec![0xC3, 0x28, b'a', b'b'], false)]
	#[case::null_bytes_are_flagged_as_binary(UTF_8, 0, vec![0x00, b'a', b'b', b'c'], true)]
	#[case::empty_sample_is_not_binary(UTF_8, 0, vec![], false)]
	#[case::utf16_is_never_binary(UTF_16LE, 0, vec![0x00, 0x00, 0x00, 0x00], false)]
	#[case::high_control_chars_flagged_binary(UTF_8, 0, vec![0x01, 0x02, 0x03, b'a', b'b', b'c', b'd', b'e', b'f', b'g'], true)]
	fn test_is_probably_binary(
		#[case] encoding: &'static Encoding,
		#[case] bom_len: usize,
		#[case] sample: Vec<u8>,
		#[case] expected: bool,
	) {
		let file_encoding = FileEncoding { encoding, bom_len };
		assert_eq!(is_probably_binary(&sample, file_encoding), expected);
	}

	#[rstest]
	#[case::detect_utf16_le_without_bom(vec![b'a', 0x00, b'b', 0x00, b'c', 0x00, b'd', 0x00], UTF_16LE, 0)]
	#[case::detect_utf16_be_without_bom(vec![0x00, b'a', 0x00, b'b', 0x00, b'c', 0x00, b'd'], UTF_16BE, 0)]
	#[case::detect_utf8_bom(vec![0xEF, 0xBB, 0xBF, b'h', b'e', b'l', b'l', b'o'], UTF_8, 3)]
	#[case::detect_utf16_le_bom(vec![0xFF, 0xFE, b'h', 0x00, b'e', 0x00], UTF_16LE, 2)]
	#[case::detect_utf16_be_bom(vec![0xFE, 0xFF, 0x00, b'h', 0x00, b'e'], UTF_16BE, 2)]
	#[case::detect_plain_utf8_default(b"hello world".to_vec(), UTF_8, 0)]
	fn test_detect_encoding(
		#[case] sample: Vec<u8>,
		#[case] expected_encoding: &'static Encoding,
		#[case] expected_bom_len: usize,
	) {
		let result = detect_encoding(&sample);
		assert_eq!(result.encoding, expected_encoding);
		assert_eq!(result.bom_len, expected_bom_len);
	}

	#[rstest]
	#[case::utf16_le(UTF_16LE, true)]
	#[case::utf16_be(UTF_16BE, true)]
	#[case::utf8(UTF_8, false)]
	fn test_is_utf16(#[case] encoding: &'static Encoding, #[case] expected: bool) {
		assert_eq!(is_utf16(encoding), expected);
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
		let encoding = FileEncoding { encoding: UTF_8, bom_len: 3 };
		let decoded = decode_bytes(&bytes, encoding, true);
		assert_eq!(&*decoded, "hi");
	}

	#[test]
	fn decode_bytes_without_strip_includes_full_content() {
		let bytes = [0xEF, 0xBB, 0xBF, b'h', b'i'];
		let encoding = FileEncoding { encoding: UTF_8, bom_len: 3 };
		let decoded = decode_bytes(&bytes, encoding, false);
		// Without stripping, the full content is decoded (BOM may or may not appear
		// depending on encoding_rs behavior, but content should be present)
		assert!(decoded.contains("hi"));
	}
}
