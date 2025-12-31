use std::{
	fs::File,
	io::{BufRead, BufReader, Read, Seek, SeekFrom},
	path::Path,
};

use anyhow::{Context as _, Result};
use memmap2::Mmap;

use super::{encoding::{self, FileEncoding}, stats::AnalysisResults, line_counter};
use crate::langs::Language;

/// Files larger than this threshold are memory-mapped instead of buffered.
const MMAP_THRESHOLD: u64 = 256 * 1024;
/// Size of sample chunks extracted from files for binary/language detection. For large files, we sample from both the start and middle of the file.
const SAMPLE_SIZE: usize = 4 * 1024;

/// Helper to create error context for file opening operations.
fn open_file_context(path: &Path) -> String {
	format!("Failed to open file {}", path.display())
}

/// Helper to create error context for memory-mapping operations.
fn mmap_file_context(path: &Path) -> String {
	format!("Failed to memory-map file {}", path.display())
}

/// Helper to create error context for file size validation.
fn file_too_large_context(path: &Path) -> String {
	format!("File too large to read: {}", path.display())
}

pub(super) trait LineSource {
	fn for_each_line<F>(&mut self, f: &mut F) -> Result<()>
	where
		F: FnMut(&[u8]);
}

pub(super) struct BufLineSource<R: BufRead> {
	reader: R,
	buffer: Vec<u8>,
}

impl<R: BufRead> BufLineSource<R> {
	pub(super) fn new(reader: R) -> Self {
		Self { reader, buffer: Vec::with_capacity(1024) }
	}
}

impl<R: BufRead> LineSource for BufLineSource<R> {
	fn for_each_line<F>(&mut self, f: &mut F) -> Result<()>
	where
		F: FnMut(&[u8]),
	{
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

pub(super) struct MmapLineSource<'a> {
	bytes: &'a [u8],
	pos: usize,
}

impl<'a> MmapLineSource<'a> {
	pub(super) const fn new(bytes: &'a [u8]) -> Self {
		Self { bytes, pos: 0 }
	}
}

impl LineSource for MmapLineSource<'_> {
	fn for_each_line<F>(&mut self, f: &mut F) -> Result<()>
	where
		F: FnMut(&[u8]),
	{
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

pub(super) enum FileSource {
	Buffered(File),
	Mapped(Mmap),
}

impl FileSource {
	pub(super) fn open(file_path: &Path, file_size: u64) -> Result<Self> {
		let file = File::open(file_path).with_context(|| open_file_context(file_path))?;
		if file_size >= MMAP_THRESHOLD {
			// SAFETY: Memory-mapping is safe here because:
			// 1. We only read from the mmap, never write.
			// 2. The file is not modified during analysis.
			// 3. The mapping is dropped before returning, so no references escape.
			let mmap = unsafe { Mmap::map(&file) }.with_context(|| mmap_file_context(file_path))?;
			Ok(Self::Mapped(mmap))
		} else {
			Ok(Self::Buffered(file))
		}
	}

	pub(super) fn sample(&mut self, file_size: u64) -> Result<Vec<u8>> {
		match self {
			Self::Buffered(file) => sample_file(file, file_size),
			Self::Mapped(mmap) => Ok(sample_from_slice(mmap)),
		}
	}

	pub(super) fn process(
		self,
		file_path: &Path,
		file_size: u64,
		results: &mut AnalysisResults,
		collect_details: bool,
		language: &'static Language,
		encoding: FileEncoding,
	) -> Result<()> {
		match self {
			Self::Buffered(file) => process_file_buffered(file_path, file, file_size, results, collect_details, language, encoding),
			Self::Mapped(mmap) => process_file_mmap(file_path, file_size, results, collect_details, language, encoding, &mmap),
		}
	}
}

fn sample_ranges(file_len: u64) -> (usize, Option<(u64, usize)>) {
	// SAFETY: SAMPLE_SIZE is a small constant (4096), so this conversion will always succeed.
	let start_len = usize::try_from(file_len.min(SAMPLE_SIZE as u64)).unwrap();
	if file_len <= SAMPLE_SIZE as u64 {
		return (start_len, None);
	}
	let mid_offset = (file_len.saturating_sub(SAMPLE_SIZE as u64)) / 2;
	// SAFETY: SAMPLE_SIZE is a small constant (4096), so this conversion will always succeed.
	let mid_len = usize::try_from((mid_offset + SAMPLE_SIZE as u64).min(file_len) - mid_offset).unwrap();
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
		// SAFETY: mid_offset is derived from file_bytes.len() which is a usize, so it must fit in usize.
		let offset = usize::try_from(mid_offset).unwrap();
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
	if encoding::is_utf16(encoding.encoding) {
		let capacity = usize::try_from(file_size).with_context(|| file_too_large_context(file_path))?;
		let mut buffer = Vec::with_capacity(capacity);
		file.read_to_end(&mut buffer)?;
		encoding::process_utf16_bytes(file_path, file_size, results, collect_details, language, encoding, &buffer);
		return Ok(());
	}
	let reader = BufReader::with_capacity(64 * 1024, file);
	let mut source = BufLineSource::new(reader);
	line_counter::process_lines(file_path, file_size, results, collect_details, language, encoding, &mut source)
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
	if encoding::is_utf16(encoding.encoding) {
		encoding::process_utf16_bytes(file_path, file_size, results, collect_details, language, encoding, file_bytes);
		return Ok(());
	}
	let mut source = MmapLineSource::new(file_bytes);
	line_counter::process_lines(file_path, file_size, results, collect_details, language, encoding, &mut source)
}
