use std::{
	cell::RefCell,
	fs::File,
	io::{ErrorKind, Read},
	ops::Deref,
	path::Path,
};

use anyhow::{Context as _, Result};
use memmap2::Mmap;

/// Files larger than this threshold are memory-mapped instead of read into memory. Mapping and unmapping each take the process's memory map lock, which the threads then queue on, so mid-sized files are cheaper to read.
const MMAP_THRESHOLD: u64 = 256 * 1024;
/// Where a file's bytes start in the read buffer. Copying a file out of the page cache to a page-aligned buffer, at the same offset in each page as its source, runs at a fraction of the usual speed on AMD's Zen 3 processors at least, and the allocator gives a large buffer a page-aligned start.
const BUFFER_OFFSET: usize = 64;
/// Size of sample chunks extracted from files for binary/language detection. For large files, we sample from both the start and middle of the file.
pub(super) const SAMPLE_SIZE: usize = 4 * 1024;

/// The start of a file, or its start and middle, for binary and language detection.
#[expect(
	clippy::large_enum_variant,
	reason = "boxing the sample would cost the allocation it is on the stack to avoid"
)]
pub(super) enum Sample<'a> {
	/// A file no longer than a sample is its own.
	Whole(&'a [u8]),
	/// The start and middle of a longer file, copied together on the stack.
	Parts([u8; SAMPLE_SIZE * 2]),
}

impl Deref for Sample<'_> {
	type Target = [u8];
	fn deref(&self) -> &[u8] {
		match self {
			Self::Whole(bytes) => bytes,
			Self::Parts(data) => data,
		}
	}
}

thread_local! {
	/// Each worker reads small files into the same buffer, so they don't cost an allocation apiece. It stays at its longest length, so nothing is zeroed twice.
	static READ_BUFFER: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

/// A file's contents: read into memory when small, memory-mapped when large.
pub(super) enum FileSource {
	/// The buffer, and how many of its bytes after [`BUFFER_OFFSET`] this file filled.
	Read(Vec<u8>, usize),
	Mapped(Mmap),
}

impl FileSource {
	/// Read or map an open `file` of `file_size` bytes; `file_path` is for error messages.
	pub(super) fn open(mut file: File, file_size: u64, file_path: &Path) -> Result<Self> {
		if file_size >= MMAP_THRESHOLD {
			// SAFETY: Memory-mapping is safe under these conditions:
			// 1. We only read from the mmap, never write.
			// 2. The caller only borrows the bytes while the `FileSource` lives.
			// 3. ASSUMPTION: The file will not be modified by external processes during analysis. This is a reasonable assumption for typical code analysis workflows where files are stable during the scan. However, concurrent modifications by other processes could cause undefined behavior.
			let mmap = unsafe { Mmap::map(&file) }
				.with_context(|| format!("Failed to memory-map file {}", file_path.display()))?;
			Ok(Self::Mapped(mmap))
		} else {
			let mut buffer = READ_BUFFER.take();
			let len = read_into(&mut file, usize::try_from(file_size).unwrap_or(0), &mut buffer)
				.with_context(|| format!("Failed to read file {}", file_path.display()))?;
			Ok(Self::Read(buffer, len))
		}
	}

	/// Read just the start of a file into `head`, returning how many bytes it holds, for files whose name matches no language and that are only worth reading in full if they begin with a shebang.
	pub(super) fn head(file: &mut File, head: &mut [u8; SAMPLE_SIZE]) -> std::io::Result<usize> {
		file.read(head)
	}
}

impl Deref for FileSource {
	type Target = [u8];
	fn deref(&self) -> &[u8] {
		match self {
			Self::Read(buffer, len) => &buffer[BUFFER_OFFSET..BUFFER_OFFSET + *len],
			Self::Mapped(mmap) => mmap,
		}
	}
}

impl Drop for FileSource {
	fn drop(&mut self) {
		if let Self::Read(buffer, _) = self {
			READ_BUFFER.set(std::mem::take(buffer));
		}
	}
}

/// Read `file` into `buffer` from [`BUFFER_OFFSET`] on, returning how many bytes it holds. A file that gives up the `size` it reported needs no further `read` to find its end, which `read_to_end` would make after several smaller ones. One reporting no size, as files in `/proc` do, is read to its end.
fn read_into(file: &mut File, size: usize, buffer: &mut Vec<u8>) -> std::io::Result<usize> {
	if size == 0 {
		buffer.resize(BUFFER_OFFSET, 0);
		return file.read_to_end(buffer);
	}
	let end = BUFFER_OFFSET + size;
	if buffer.len() < end {
		buffer.resize(end, 0);
	}
	let mut len = 0;
	while len < size {
		match file.read(&mut buffer[BUFFER_OFFSET + len..end]) {
			Ok(0) => break,
			Ok(read) => len += read,
			Err(err) if err.kind() == ErrorKind::Interrupted => {}
			Err(err) => return Err(err),
		}
	}
	Ok(len)
}

/// Where the middle sample of a file of `len` bytes starts, or `None` if the file is no longer than one sample. The offset is even, keeping UTF-16 code units whole.
const fn middle(len: usize) -> Option<usize> {
	if len <= SAMPLE_SIZE { None } else { Some(((len - SAMPLE_SIZE) / 2) & !1) }
}

/// The start and middle of a file's contents, for binary and language detection.
pub(super) fn sample(file_bytes: &[u8]) -> Sample<'_> {
	let Some(middle) = middle(file_bytes.len()) else {
		return Sample::Whole(file_bytes);
	};
	let mut data = [0; SAMPLE_SIZE * 2];
	data[..SAMPLE_SIZE].copy_from_slice(&file_bytes[..SAMPLE_SIZE]);
	data[SAMPLE_SIZE..].copy_from_slice(&file_bytes[middle..middle + SAMPLE_SIZE]);
	Sample::Parts(data)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn middle_sample_starts_halfway_at_an_even_offset() {
		assert_eq!(middle(100), None);
		assert_eq!(middle(SAMPLE_SIZE), None);
		assert_eq!(middle(SAMPLE_SIZE + 3), Some(0));
		assert_eq!(middle(100_000), Some(47_952));
	}

	#[test]
	fn read_into_stops_at_the_size_and_the_end() {
		let path = std::env::temp_dir().join(format!("codestats_read_into_test_{}", std::process::id()));
		std::fs::write(&path, b"hello world").unwrap();
		// The buffer's longer than the file, as it is after a bigger one, and its old contents don't show.
		let mut buffer = vec![b'x'; BUFFER_OFFSET + 64];
		let mut read = |size| {
			let len = read_into(&mut File::open(&path).unwrap(), size, &mut buffer).unwrap();
			buffer[BUFFER_OFFSET..BUFFER_OFFSET + len].to_vec()
		};
		assert_eq!(read(11), b"hello world");
		// It shrank since the size was taken, or grew, or reports no size.
		assert_eq!(read(100), b"hello world");
		assert_eq!(read(5), b"hello");
		assert_eq!(read(0), b"hello world");
		std::fs::remove_file(&path).unwrap();
	}

	#[test]
	fn test_sample_from_slice_small() {
		let data: Vec<u8> = (0..100).collect();
		let samples = sample(&data);
		assert_eq!(samples.len(), 100);
		assert_eq!(&samples[..], &data[..]);
	}

	#[test]
	fn test_sample_from_slice_large() {
		let data: Vec<u8> = (0u8..=255).cycle().take(10_000).collect();
		let samples = sample(&data);
		assert_eq!(&samples[..SAMPLE_SIZE], &data[..SAMPLE_SIZE]);
		assert_eq!(&samples[SAMPLE_SIZE..], &data[2952..2952 + SAMPLE_SIZE]);
	}
}
