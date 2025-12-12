use std::{
	fs::File,
	io::{BufRead, BufReader, Read, Seek, SeekFrom},
	path::Path,
	str,
};

use anyhow::{Context as _, Result};
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
		line_bytes: &[u8],
		lang_info: Option<&Language>,
		comment_state: &mut CommentState,
		is_first_line: bool,
	) {
		let line_type = if let Ok(line) = str::from_utf8(line_bytes) {
			line_classifier::classify_line(line, lang_info, comment_state, is_first_line)
		} else {
			let line = String::from_utf8_lossy(line_bytes);
			line_classifier::classify_line(line.as_ref(), lang_info, comment_state, is_first_line)
		};
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

pub fn process_file(file_path: &Path, results: &mut AnalysisResults, collect_details: bool) -> Result<()> {
	let filename = file_path.file_name().and_then(|name| name.to_str()).context("Invalid UTF-8 in file name")?;
	let metadata =
		file_path.metadata().with_context(|| format!("Failed to read metadata for {}", file_path.display()))?;
	let file_size = metadata.len();
	let language_from_name = langs::detect_language_info(filename, None);
	if file_size == 0 {
		if let Some(language) = language_from_name {
			let contribution = FileContribution::new(0, 0, 0, 0, 0, file_size);
			let file_stats =
				collect_details.then(|| FileStats::new(file_path.display().to_string(), 0, 0, 0, 0, 0, file_size));
			results.add_file_stats(language, contribution, file_stats);
		}
		return Ok(());
	}
	if file_size >= MMAP_THRESHOLD {
		process_file_mmap(file_path, filename, file_size, results, collect_details)
	} else {
		process_file_buffered(file_path, filename, file_size, results, collect_details)
	}
}

fn detect_language_from_samples(filename: &str, samples: &[u8]) -> Option<&'static Language> {
	if is_probably_binary(samples) {
		return None;
	}
	let mut sample_text_owned = None;
	let sample_str = str::from_utf8(samples).map_or_else(
		|_| {
			let owned = String::from_utf8_lossy(samples).into_owned();
			sample_text_owned = Some(owned);
			sample_text_owned.as_deref()
		},
		Some,
	);
	langs::detect_language_info(filename, sample_str)
}

fn sample_file(file: &mut File, file_size: u64) -> Result<Vec<u8>> {
	let mut buffer = Vec::with_capacity(SAMPLE_SIZE * 2);
	let mut chunk = [0u8; SAMPLE_SIZE];
	let read_start = file.read(&mut chunk)?;
	buffer.extend_from_slice(&chunk[..read_start]);
	if file_size > SAMPLE_SIZE as u64 {
		let mid_offset = file_size.saturating_sub(SAMPLE_SIZE as u64) / 2;
		file.seek(SeekFrom::Start(mid_offset))?;
		let read_mid = file.read(&mut chunk)?;
		buffer.extend_from_slice(&chunk[..read_mid]);
	}
	// Reset for actual reading.
	file.rewind()?;
	Ok(buffer)
}

fn process_file_buffered(
	file_path: &Path,
	filename: &str,
	file_size: u64,
	results: &mut AnalysisResults,
	collect_details: bool,
) -> Result<()> {
	let file = File::open(file_path).with_context(|| format!("Failed to open file {}", file_path.display()))?;
	let mut file = file;
	let sample_bytes = sample_file(&mut file, file_size)?;
	let Some(language) = detect_language_from_samples(filename, &sample_bytes) else { return Ok(()) };
	let reader = BufReader::with_capacity(64 * 1024, file);
	let mut source = BufLineSource::new(reader);
	process_lines(file_path, file_size, results, collect_details, language, &mut source)
}

fn process_file_mmap(
	file_path: &Path,
	filename: &str,
	file_size: u64,
	results: &mut AnalysisResults,
	collect_details: bool,
) -> Result<()> {
	let file = File::open(file_path).with_context(|| format!("Failed to open file {}", file_path.display()))?;
	// SAFETY: We only read from the mmap and don't modify the underlying read-only file during analysis.
	let mmap =
		unsafe { Mmap::map(&file) }.with_context(|| format!("Failed to memory-map file {}", file_path.display()))?;
	let file_bytes = &*mmap;
	let mut samples = Vec::with_capacity(SAMPLE_SIZE * 2);
	let start_len = file_bytes.len().min(SAMPLE_SIZE);
	samples.extend_from_slice(&file_bytes[..start_len]);
	if file_bytes.len() > SAMPLE_SIZE {
		let mid_offset = (file_bytes.len().saturating_sub(SAMPLE_SIZE)) / 2;
		let mid_end = (mid_offset + SAMPLE_SIZE).min(file_bytes.len());
		samples.extend_from_slice(&file_bytes[mid_offset..mid_end]);
	}
	let Some(language) = detect_language_from_samples(filename, &samples) else { return Ok(()) };
	let mut source = MmapLineSource::new(file_bytes);
	process_lines(file_path, file_size, results, collect_details, language, &mut source)
}

fn process_lines(
	file_path: &Path,
	file_size: u64,
	results: &mut AnalysisResults,
	collect_details: bool,
	language: &'static Language,
	source: &mut dyn LineSource,
) -> Result<()> {
	let mut line_counts = LineCounts::default();
	let mut comment_state = CommentState::new();
	let mut is_first_line = true;
	source.for_each_line(&mut |line_bytes| {
		line_counts.classify_and_count(line_bytes, Some(language), &mut comment_state, is_first_line);
		is_first_line = false;
	})?;
	let LineCounts { total, code, comment, blank, shebang } = line_counts;
	let contribution = FileContribution::new(total, code, comment, blank, shebang, file_size);
	let file_stats = collect_details
		.then(|| FileStats::new(file_path.display().to_string(), total, code, comment, blank, shebang, file_size));
	results.add_file_stats(language, contribution, file_stats);
	Ok(())
}

fn is_probably_binary(sample: &[u8]) -> bool {
	if sample.is_empty() {
		return false;
	}
	if sample.contains(&0) {
		return true;
	}
	if str::from_utf8(sample).is_ok() {
		return false;
	}
	let non_text = sample.iter().filter(|b| matches!(**b, 0x00..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F | 0x7F)).count();
	non_text * 100 / sample.len() > BINARY_THRESHOLD_PERCENT
}
