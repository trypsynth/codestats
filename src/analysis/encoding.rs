use std::{borrow::Cow, path::Path};

use encoding_rs::{CoderResult, Decoder, Encoding, UTF_16BE, UTF_16LE, UTF_8};
use memchr::memchr;

use super::{line_counter::LineCounts, line_classifier::CommentState, stats::AnalysisResults};
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
		FileEncoding { encoding: UTF_8, bom_len: 0 }
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
	let (_, _, had_errors) = encoding.encoding.decode(sample);
	had_errors
}

pub(super) fn is_utf16(encoding: &'static Encoding) -> bool {
	encoding == UTF_16LE || encoding == UTF_16BE
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
	line_counts: &mut super::line_counter::LineCounts,
	comment_state: &mut CommentState,
	is_first_line: &mut bool,
	flush_final: bool,
) {
	while let Some(pos) = memchr(b'\n', pending.as_bytes()) {
		let line_end = pos + 1;
		{
			let line = &pending[..line_end];
			line_counts.classify_and_count(line, Some(language), comment_state, *is_first_line);
			*is_first_line = false;
		}
		pending.drain(..line_end);
	}
	if flush_final && !pending.is_empty() {
		line_counts.classify_and_count(pending.as_str(), Some(language), comment_state, *is_first_line);
		*is_first_line = false;
		pending.clear();
	}
}
