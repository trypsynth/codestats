use std::{
	borrow::Cow,
	fs::File,
	io::{BufRead, BufReader, Read, Seek, SeekFrom},
	path::Path,
};

use anyhow::{Context as _, Result};
use encoding_rs::{Encoding, UTF_8, UTF_16BE, UTF_16LE};
use memmap2::Mmap;

use super::{
	line_classifier::{self, CommentState, LineType},
	stats::{AnalysisResults, FileContribution, FileStats},
};
use crate::langs::{self, Language};

#[derive(Default)]
struct LineCounts {
	total: u64,
	code: u64,
	comment: u64,
	blank: u64,
	shebang: u64,
}

impl LineCounts {
	pub(crate) fn classify_and_count(
		&mut self,
		line: &str,
		lang_info: Option<&Language>,
		comment_state: &mut CommentState,
		is_first_line: bool,
	) {
		let line_type = line_classifier::classify_line(line, lang_info, comment_state, is_first_line);
		match line_type {
			LineType::Code => self.code += 1,
			LineType::Comment => self.comment += 1,
			LineType::Blank => self.blank += 1,
			LineType::Shebang => self.shebang += 1,
		}
		self.total += 1;
	}
}

trait LineSource {
	fn for_each_line(&mut self, f: &mut dyn FnMut(&[u8])) -> Result<()>;
}

struct BufLineSource<R: BufRead> {
	reader: R,
	buffer: Vec<u8>,
}

impl<R: BufRead> BufLineSource<R> {
	fn new(reader: R) -> Self {
		Self { reader, buffer: Vec::with_capacity(1024) }
	}
}

impl<R: BufRead> LineSource for BufLineSource<R> {
	fn for_each_line(&mut self, f: &mut dyn FnMut(&[u8])) -> Result<()> {
		loop {
			self.buffer.clear();
			let bytes_read = self.reader.read_until(b'\n', &mut self.buffer)?;
			if bytes_read == 0 {
				break;
			}
			f(&self.buffer);
		}
		Ok(())
	}
}

struct MmapLineSource<'a> {
	bytes: &'a [u8],
	pos: usize,
}

impl<'a> MmapLineSource<'a> {
	const fn new(bytes: &'a [u8]) -> Self {
		Self { bytes, pos: 0 }
	}
}

impl LineSource for MmapLineSource<'_> {
	fn for_each_line(&mut self, f: &mut dyn FnMut(&[u8])) -> Result<()> {
		while self.pos < self.bytes.len() {
			let line_end =
				memchr::memchr(b'\n', &self.bytes[self.pos..]).map_or(self.bytes.len(), |offset| self.pos + offset + 1);
			let line_bytes = &self.bytes[self.pos..line_end];
			f(line_bytes);
			self.pos = line_end;
		}
		Ok(())
	}
}

const MMAP_THRESHOLD: u64 = 256 * 1024; // 256KB threshold
const SAMPLE_SIZE: usize = 4 * 1024; // 4KB sample for binary/language detection
const BINARY_THRESHOLD_PERCENT: usize = 20; // 20% non-text bytes threshold

#[derive(Clone, Copy)]
struct FileEncoding {
	encoding: &'static Encoding,
	bom_len: usize,
}

enum FileSource {
	Buffered(File),
	Mapped(Mmap),
}

impl FileSource {
	fn open(file_path: &Path, file_size: u64) -> Result<Self> {
		if file_size >= MMAP_THRESHOLD {
			let file = File::open(file_path).with_context(|| format!("Failed to open file {}", file_path.display()))?;
			// SAFETY: We only read from the mmap and don't modify the underlying read-only file during analysis.
			let mmap = unsafe { Mmap::map(&file) }
				.with_context(|| format!("Failed to memory-map file {}", file_path.display()))?;
			Ok(Self::Mapped(mmap))
		} else {
			let file = File::open(file_path).with_context(|| format!("Failed to open file {}", file_path.display()))?;
			Ok(Self::Buffered(file))
		}
	}

	fn sample(&mut self, file_size: u64) -> Result<Vec<u8>> {
		match self {
			Self::Buffered(file) => sample_file(file, file_size),
			Self::Mapped(mmap) => Ok(sample_from_slice(mmap)),
		}
	}

	fn process(
		self,
		file_path: &Path,
		file_size: u64,
		results: &mut AnalysisResults,
		collect_details: bool,
		language: &'static Language,
		encoding: FileEncoding,
	) -> Result<()> {
		match self {
			Self::Buffered(file) => {
				process_file_buffered(file_path, file, file_size, results, collect_details, language, encoding)
			}
			Self::Mapped(mmap) => {
				process_file_mmap(file_path, file_size, results, collect_details, language, encoding, &mmap)
			}
		}
	}
}

pub fn process_file(file_path: &Path, results: &mut AnalysisResults, collect_details: bool) -> Result<()> {
	let filename_os = file_path.file_name().context("Missing file name")?;
	let filename = filename_os.to_string_lossy();
	let metadata =
		file_path.metadata().with_context(|| format!("Failed to read metadata for {}", file_path.display()))?;
	let file_size = metadata.len();
	let language_from_name = langs::detect_language_info(&filename, None);
	if file_size == 0 {
		if let Some(language) = language_from_name {
			let contribution = FileContribution::new(0, 0, 0, 0, 0, file_size);
			let file_stats =
				collect_details.then(|| FileStats::new(file_path.display().to_string(), 0, 0, 0, 0, 0, file_size));
			results.add_file_stats(language, contribution, file_stats);
		}
		return Ok(());
	}
	let mut source = FileSource::open(file_path, file_size)?;
	let sample_bytes = source.sample(file_size)?;
	let Some((language, encoding)) = detect_language_and_encoding(&filename, &sample_bytes) else { return Ok(()) };
	source.process(file_path, file_size, results, collect_details, language, encoding)
}

fn detect_language_from_samples(filename: &str, samples: &[u8], encoding: FileEncoding) -> Option<&'static Language> {
	if is_probably_binary(samples, encoding) {
		return None;
	}
	let decoded = decode_bytes(samples, encoding, true);
	langs::detect_language_info(filename, Some(decoded.as_ref()))
}

fn sample_ranges(file_len: u64) -> (usize, Option<(u64, usize)>) {
	let start_len = usize::try_from(file_len.min(SAMPLE_SIZE as u64)).expect("sample size is bounded by SAMPLE_SIZE");
	if file_len <= SAMPLE_SIZE as u64 {
		return (start_len, None);
	}
	let mid_offset = (file_len.saturating_sub(SAMPLE_SIZE as u64)) / 2;
	let mid_len = usize::try_from((mid_offset + SAMPLE_SIZE as u64).min(file_len) - mid_offset)
		.expect("sample size is bounded by SAMPLE_SIZE");
	(start_len, Some((mid_offset, mid_len)))
}

fn sample_file(file: &mut File, file_size: u64) -> Result<Vec<u8>> {
	let mut buffer = Vec::with_capacity(SAMPLE_SIZE * 2);
	let mut chunk = [0u8; SAMPLE_SIZE];
	let (start_len, mid_range) = sample_ranges(file_size);
	let read_start = file.read(&mut chunk[..start_len])?;
	buffer.extend_from_slice(&chunk[..read_start]);
	if let Some((mid_offset, mid_len)) = mid_range {
		file.seek(SeekFrom::Start(mid_offset))?;
		let read_mid = file.read(&mut chunk[..mid_len])?;
		buffer.extend_from_slice(&chunk[..read_mid]);
	}
	// Reset for actual reading.
	file.rewind()?;
	Ok(buffer)
}

fn sample_from_slice(file_bytes: &[u8]) -> Vec<u8> {
	let mut samples = Vec::with_capacity(SAMPLE_SIZE * 2);
	let (start_len, mid_range) = sample_ranges(file_bytes.len() as u64);
	samples.extend_from_slice(&file_bytes[..start_len]);
	if let Some((mid_offset, mid_len)) = mid_range {
		let offset = usize::try_from(mid_offset).expect("mid offset is derived from slice length");
		samples.extend_from_slice(&file_bytes[offset..offset + mid_len]);
	}
	samples
}

fn process_file_buffered(
	file_path: &Path,
	mut file: File,
	file_size: u64,
	results: &mut AnalysisResults,
	collect_details: bool,
	language: &'static Language,
	encoding: FileEncoding,
) -> Result<()> {
	if is_utf16(encoding.encoding) {
		let capacity =
			usize::try_from(file_size).with_context(|| format!("File too large to read: {}", file_path.display()))?;
		let mut buffer = Vec::with_capacity(capacity);
		file.read_to_end(&mut buffer)?;
		process_utf16_bytes(file_path, file_size, results, collect_details, language, encoding, &buffer)?;
		return Ok(());
	}
	let reader = BufReader::with_capacity(64 * 1024, file);
	let mut source = BufLineSource::new(reader);
	process_lines(file_path, file_size, results, collect_details, language, encoding, &mut source)
}

fn process_file_mmap(
	file_path: &Path,
	file_size: u64,
	results: &mut AnalysisResults,
	collect_details: bool,
	language: &'static Language,
	encoding: FileEncoding,
	mmap: &Mmap,
) -> Result<()> {
	let file_bytes = mmap.as_ref();
	if is_utf16(encoding.encoding) {
		process_utf16_bytes(file_path, file_size, results, collect_details, language, encoding, file_bytes)?;
		return Ok(());
	}
	let mut source = MmapLineSource::new(file_bytes);
	process_lines(file_path, file_size, results, collect_details, language, encoding, &mut source)
}

fn detect_language_and_encoding(filename: &str, samples: &[u8]) -> Option<(&'static Language, FileEncoding)> {
	let encoding = detect_encoding(samples);
	detect_language_from_samples(filename, samples, encoding).map(|language| (language, encoding))
}

fn process_lines(
	file_path: &Path,
	file_size: u64,
	results: &mut AnalysisResults,
	collect_details: bool,
	language: &'static Language,
	encoding: FileEncoding,
	source: &mut dyn LineSource,
) -> Result<()> {
	let mut is_first_line = true;
	let line_counts = count_lines_with(
		|handle| {
			source.for_each_line(&mut |line_bytes| {
				let decoded = decode_bytes(line_bytes, encoding, is_first_line);
				handle(decoded.as_ref(), is_first_line);
				is_first_line = false;
			})
		},
		language,
	)?;
	finish_file_stats(file_path, file_size, results, collect_details, language, &line_counts);
	Ok(())
}

fn process_utf16_bytes(
	file_path: &Path,
	file_size: u64,
	results: &mut AnalysisResults,
	collect_details: bool,
	language: &'static Language,
	encoding: FileEncoding,
	bytes: &[u8],
) -> Result<()> {
	let decoded = decode_bytes(bytes, encoding, true);
	let mut is_first_line = true;
	let line_counts = count_lines_with(
		|handle| {
			for line in decoded.lines() {
				handle(line, is_first_line);
				is_first_line = false;
			}
			Ok(())
		},
		language,
	)?;
	finish_file_stats(file_path, file_size, results, collect_details, language, &line_counts);
	Ok(())
}

fn finish_file_stats(
	file_path: &Path,
	file_size: u64,
	results: &mut AnalysisResults,
	collect_details: bool,
	language: &'static Language,
	line_counts: &LineCounts,
) {
	let total = line_counts.total;
	let code = line_counts.code;
	let comment = line_counts.comment;
	let blank = line_counts.blank;
	let shebang = line_counts.shebang;
	let contribution = FileContribution::new(total, code, comment, blank, shebang, file_size);
	let file_stats = collect_details
		.then(|| FileStats::new(file_path.display().to_string(), total, code, comment, blank, shebang, file_size));
	results.add_file_stats(language, contribution, file_stats);
}

fn count_lines_with(
	mut for_each: impl FnMut(&mut dyn FnMut(&str, bool)) -> Result<()>,
	language: &'static Language,
) -> Result<LineCounts> {
	let mut line_counts = LineCounts::default();
	let mut comment_state = CommentState::new();
	for_each(&mut |line, is_first_line| {
		line_counts.classify_and_count(line, Some(language), &mut comment_state, is_first_line);
	})?;
	Ok(line_counts)
}

fn detect_encoding(samples: &[u8]) -> FileEncoding {
	if let Some((encoding, bom_len)) = Encoding::for_bom(samples) {
		FileEncoding { encoding, bom_len }
	} else {
		FileEncoding { encoding: UTF_8, bom_len: 0 }
	}
}

fn decode_bytes(bytes: &[u8], encoding: FileEncoding, strip_bom: bool) -> Cow<'_, str> {
	let mut slice = bytes;
	if strip_bom && encoding.bom_len > 0 && slice.len() >= encoding.bom_len {
		slice = &slice[encoding.bom_len..];
	}
	let (decoded, _, _) = encoding.encoding.decode(slice);
	decoded
}

fn is_probably_binary(sample: &[u8], encoding: FileEncoding) -> bool {
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

fn is_utf16(encoding: &'static Encoding) -> bool {
	encoding == UTF_16LE || encoding == UTF_16BE
}
