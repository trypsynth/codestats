use std::{
	fs::File,
	io::{BufRead, BufReader},
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

pub struct LineCounts {
	total: u64,
	code: u64,
	comment: u64,
	blank: u64,
	shebang: u64,
}

impl LineCounts {
	pub(crate) const fn new() -> Self {
		Self { total: 0, code: 0, comment: 0, blank: 0, shebang: 0 }
	}

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

	pub(crate) const fn total(&self) -> u64 {
		self.total
	}

	pub(crate) const fn code(&self) -> u64 {
		self.code
	}

	pub(crate) const fn comment(&self) -> u64 {
		self.comment
	}

	pub(crate) const fn blank(&self) -> u64 {
		self.blank
	}

	pub(crate) const fn shebang(&self) -> u64 {
		self.shebang
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

pub fn process_file(file_path: &Path, results: &mut AnalysisResults, collect_details: bool) -> Result<()> {
	let filename = file_path.file_name().and_then(|name| name.to_str()).context("Invalid UTF-8 in file name")?;
	let metadata =
		file_path.metadata().with_context(|| format!("Failed to read metadata for {}", file_path.display()))?;
	let file_size = metadata.len();
	if file_size == 0 {
		return Ok(());
	}
	if file_size >= MMAP_THRESHOLD {
		process_file_mmap(file_path, filename, file_size, results, collect_details)
	} else {
		process_file_buffered(file_path, filename, file_size, results, collect_details)
	}
}

fn process_file_buffered(
	file_path: &Path,
	filename: &str,
	file_size: u64,
	results: &mut AnalysisResults,
	collect_details: bool,
) -> Result<()> {
	let file = File::open(file_path).with_context(|| format!("Failed to open file {}", file_path.display()))?;
	let mut reader = BufReader::with_capacity(64 * 1024, file);
	let sample_bytes = {
		let buf = reader.fill_buf()?;
		let len = buf.len().min(4 * 1024);
		buf[..len].to_vec()
	};
	if is_probably_binary(&sample_bytes) {
		return Ok(());
	}
	let sample_str = str::from_utf8(&sample_bytes).ok();
	let Some(language) = langs::detect_language_info(filename, sample_str) else { return Ok(()) };
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
	let sample_size = file_bytes.len().min(4 * 1024);
	let sample_bytes = &file_bytes[..sample_size];
	if is_probably_binary(sample_bytes) {
		return Ok(());
	}
	let sample_str = str::from_utf8(sample_bytes).ok();
	let Some(language) = langs::detect_language_info(filename, sample_str) else { return Ok(()) };
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
	let mut line_counts = LineCounts::new();
	let mut comment_state = CommentState::new();
	let mut is_first_line = true;
	source.for_each_line(&mut |line_bytes| {
		line_counts.classify_and_count(line_bytes, Some(language), &mut comment_state, is_first_line);
		is_first_line = false;
	})?;
	let contribution = FileContribution::new(
		line_counts.total(),
		line_counts.code(),
		line_counts.comment(),
		line_counts.blank(),
		line_counts.shebang(),
		file_size,
	);
	let file_stats = collect_details.then(|| {
		FileStats::new(
			file_path.display().to_string(),
			line_counts.total(),
			line_counts.code(),
			line_counts.comment(),
			line_counts.blank(),
			line_counts.shebang(),
			file_size,
		)
	});
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
	let non_text = sample.iter().filter(|b| matches!(**b, 0x00..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F | 0x7F)).count();
	// Consider binary if more than 20% of the sampled bytes look non-textual.
	non_text * 5 > sample.len()
}
