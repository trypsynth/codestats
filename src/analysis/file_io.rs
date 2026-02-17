use std::{
	fs::File,
	io::{BufRead, BufReader, Read, Seek, SeekFrom},
	path::Path,
};

use anyhow::{Context as _, Result};
use memmap2::Mmap;

use super::{
	encoding::{self, FileEncoding},
	line_counter,
	stats::AnalysisResults,
};
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
			// SAFETY: Memory-mapping is safe under these conditions:
			// 1. We only read from the mmap, never write.
			// 2. The mapping is dropped before returning, so no references escape.
			// 3. ASSUMPTION: The file will not be modified by external processes during analysis. This is a reasonable assumption for typical code analysis workflows where files are stable during the scan. However, concurrent modifications by other processes could cause undefined behavior.
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
			Self::Buffered(file) => {
				process_file_buffered(file_path, file, file_size, results, collect_details, language, encoding)
			}
			Self::Mapped(mmap) => {
				process_file_mmap(file_path, file_size, results, collect_details, language, encoding, &mmap)
			}
		}
	}
}

fn sample_ranges(file_len: u64) -> (usize, Option<(u64, usize)>) {
	// SAFETY: SAMPLE_SIZE is a small constant (4096), so this conversion will always succeed.
	let start_len = usize::try_from(file_len.min(SAMPLE_SIZE as u64)).unwrap();
	if file_len <= SAMPLE_SIZE as u64 {
		return (start_len, None);
	}
	let mut mid_offset = (file_len.saturating_sub(SAMPLE_SIZE as u64)) / 2;
	if mid_offset % 2 == 1 {
		// Keep UTF-16 code units aligned when sampling from the middle.
		mid_offset = mid_offset.saturating_sub(1);
	}
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
	file: File,
	file_size: u64,
	results: &mut AnalysisResults,
	collect_details: bool,
	language: &'static Language,
	encoding: FileEncoding,
) -> Result<()> {
	if encoding::is_utf16(encoding.encoding) {
		let mut reader = BufReader::with_capacity(64 * 1024, file);
		return encoding::process_utf16_stream(
			file_path,
			file_size,
			results,
			collect_details,
			language,
			encoding,
			&mut reader,
		);
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

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;

	#[rstest]
	#[case::empty(b"" as &[u8], vec![])]
	#[case::single_no_newline(b"hello" as &[u8], vec![b"hello".to_vec()])]
	#[case::single_with_newline(b"hello\n" as &[u8], vec![b"hello\n".to_vec()])]
	#[case::multiple(b"line1\nline2\nline3" as &[u8], vec![b"line1\n".to_vec(), b"line2\n".to_vec(), b"line3".to_vec()])]
	#[case::crlf(b"line1\r\nline2\r\n" as &[u8], vec![b"line1\r\n".to_vec(), b"line2\r\n".to_vec()])]
	fn test_mmap_line_source(#[case] data: &[u8], #[case] expected: Vec<Vec<u8>>) {
		let mut source = MmapLineSource::new(data);
		let mut lines = Vec::new();
		source.for_each_line(&mut |line| lines.push(line.to_vec())).unwrap();
		assert_eq!(lines, expected);
	}

	#[rstest]
	#[case::small(100, 100, true)]
	#[case::exact(SAMPLE_SIZE as u64, SAMPLE_SIZE, true)]
	fn test_sample_ranges_no_mid(#[case] file_len: u64, #[case] expected_start: usize, #[case] mid_is_none: bool) {
		let (start_len, mid) = sample_ranges(file_len);
		assert_eq!(start_len, expected_start);
		assert_eq!(mid.is_none(), mid_is_none);
	}

	#[test]
	fn test_sample_ranges_large_file() {
		let file_size = 100_000u64;
		let (start_len, mid) = sample_ranges(file_size);
		assert_eq!(start_len, SAMPLE_SIZE);
		let (mid_offset, mid_len) = mid.expect("should have mid range");
		// Mid offset should be roughly in the middle
		assert!(mid_offset > 0);
		assert!(mid_offset < file_size - SAMPLE_SIZE as u64);
		assert_eq!(mid_len, SAMPLE_SIZE);
		// Offset should be even for UTF-16 alignment
		assert_eq!(mid_offset % 2, 0);
	}

	#[test]
	fn test_sample_from_slice_small() {
		let data: Vec<u8> = (0..100).collect();
		let samples = sample_from_slice(&data);
		assert_eq!(samples.len(), 100);
		assert_eq!(&samples[..], &data[..]);
	}

	#[test]
	fn test_sample_from_slice_large() {
		let data: Vec<u8> = (0u8..=255).cycle().take(10_000).collect();
		let samples = sample_from_slice(&data);
		// Should have start sample + mid sample
		assert!(samples.len() > SAMPLE_SIZE);
		assert!(samples.len() <= SAMPLE_SIZE * 2);
		// First SAMPLE_SIZE bytes should match
		assert_eq!(&samples[..SAMPLE_SIZE], &data[..SAMPLE_SIZE]);
	}

	#[test]
	fn test_buf_line_source_multiple_lines() {
		use std::io::Cursor;
		let data = b"line1\nline2\nline3";
		let reader = std::io::BufReader::new(Cursor::new(data));
		let mut source = BufLineSource::new(reader);
		let mut lines = Vec::new();
		source.for_each_line(&mut |line| lines.push(line.to_vec())).unwrap();
		assert_eq!(lines.len(), 3);
		assert_eq!(lines[0], b"line1\n");
		assert_eq!(lines[1], b"line2\n");
		assert_eq!(lines[2], b"line3");
	}
}
