use std::{cell::RefCell, fs::File, io::Read, ops::Deref, path::Path};

use anyhow::{Context as _, Result};
use memmap2::Mmap;

/// Files larger than this threshold are memory-mapped instead of read into memory.
const MMAP_THRESHOLD: u64 = 64 * 1024;
/// Size of sample chunks extracted from files for binary/language detection. For large files, we sample from both the start and middle of the file.
const SAMPLE_SIZE: usize = 4 * 1024;
const MAX_SAMPLE: usize = SAMPLE_SIZE * 2;

/// Stack-allocated sample buffer; avoids a heap allocation per file during detection.
pub(super) struct SampleBuf {
	data: [u8; MAX_SAMPLE],
	len: usize,
}

impl SampleBuf {
	const fn new() -> Self {
		Self { data: [0u8; MAX_SAMPLE], len: 0 }
	}
}

impl Deref for SampleBuf {
	type Target = [u8];
	fn deref(&self) -> &[u8] {
		&self.data[..self.len]
	}
}

thread_local! {
	/// Each worker reads small files into the same buffer, so they don't cost an allocation apiece.
	static READ_BUFFER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// A file's contents: read into memory when small, memory-mapped when large.
pub(super) enum FileSource {
	Read(Vec<u8>),
	Mapped(Mmap),
}

impl FileSource {
	/// Read or map an open `file` of `file_size` bytes; `file_path` is for error messages.
	pub(super) fn open(mut file: File, file_size: u64, file_path: &Path) -> Result<Self> {
		if file_size >= MMAP_THRESHOLD {
			// SAFETY: Memory-mapping is safe under these conditions:
			// 1. We only read from the mmap, never write.
			// 2. The mapping is dropped before returning, so no references escape.
			// 3. ASSUMPTION: The file will not be modified by external processes during analysis. This is a reasonable assumption for typical code analysis workflows where files are stable during the scan. However, concurrent modifications by other processes could cause undefined behavior.
			let mmap = unsafe { Mmap::map(&file) }
				.with_context(|| format!("Failed to memory-map file {}", file_path.display()))?;
			Ok(Self::Mapped(mmap))
		} else {
			let mut buffer = READ_BUFFER.take();
			buffer.clear();
			// The size is known, so reserve it and read through `Take`, which skips the `fstat` and `lseek` that `File::read_to_end` makes to size its buffer.
			buffer.reserve(usize::try_from(file_size).unwrap_or(0) + 1);
			Read::by_ref(&mut file)
				.take(u64::MAX)
				.read_to_end(&mut buffer)
				.with_context(|| format!("Failed to read file {}", file_path.display()))?;
			Ok(Self::Read(buffer))
		}
	}

	/// Read just the start of a file, for files whose name matches no language and that are only worth reading in full if they begin with a shebang.
	pub(super) fn head(file: &mut File) -> std::io::Result<SampleBuf> {
		let mut sample = SampleBuf::new();
		sample.len = file.read(&mut sample.data[..SAMPLE_SIZE])?;
		Ok(sample)
	}

	/// The start and middle of the file, for binary and language detection.
	pub(super) fn sample(&self) -> SampleBuf {
		sample_from_slice(self)
	}
}

impl Deref for FileSource {
	type Target = [u8];
	fn deref(&self) -> &[u8] {
		match self {
			Self::Read(buffer) => buffer,
			Self::Mapped(mmap) => mmap,
		}
	}
}

impl Drop for FileSource {
	fn drop(&mut self) {
		if let Self::Read(buffer) = self {
			READ_BUFFER.set(std::mem::take(buffer));
		}
	}
}

fn sample_ranges(file_len: u64) -> (usize, Option<(u64, usize)>) {
	let start_len = usize::try_from(file_len.min(SAMPLE_SIZE as u64))
		.expect("SAMPLE_SIZE is 4096, well within usize range on any supported platform");
	if file_len <= SAMPLE_SIZE as u64 {
		return (start_len, None);
	}
	let mut mid_offset = (file_len.saturating_sub(SAMPLE_SIZE as u64)) / 2;
	if mid_offset % 2 == 1 {
		// Keep UTF-16 code units aligned when sampling from the middle.
		mid_offset = mid_offset.saturating_sub(1);
	}
	let mid_len = usize::try_from((mid_offset + SAMPLE_SIZE as u64).min(file_len) - mid_offset)
		.expect("result is bounded by SAMPLE_SIZE (4096), well within usize range");
	(start_len, Some((mid_offset, mid_len)))
}

fn sample_from_slice(file_bytes: &[u8]) -> SampleBuf {
	let mut buf = SampleBuf::new();
	let (start_len, mid_range) = sample_ranges(file_bytes.len() as u64);
	buf.data[..start_len].copy_from_slice(&file_bytes[..start_len]);
	buf.len = start_len;
	if let Some((mid_offset, mid_len)) = mid_range {
		let offset =
			usize::try_from(mid_offset).expect("mid_offset derives from file_bytes.len() which is already a usize");
		buf.data[buf.len..buf.len + mid_len].copy_from_slice(&file_bytes[offset..offset + mid_len]);
		buf.len += mid_len;
	}
	buf
}

#[cfg(test)]
mod tests {
	use rstest::rstest;

	use super::*;

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
}
