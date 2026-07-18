use std::{
	fs,
	io::{self, Cursor},
	path::Path,
};

use crate::error::AppError;

/// Reject uploads larger than this before we even try to decompress them.
pub const MAX_UPLOAD_BYTES: usize = 50 * 1024 * 1024;
/// Cap total decompressed bytes written to disk, as a zip-bomb guard.
const MAX_EXTRACTED_BYTES: u64 = 250 * 1024 * 1024;

/// Extract `bytes` (a zip archive) into `dest`, which must already exist and be empty.
///
/// Rejects entries whose path would escape `dest` (zip-slip) and bails out once the
/// cumulative decompressed size exceeds [`MAX_EXTRACTED_BYTES`].
pub fn extract(bytes: &[u8], dest: &Path) -> Result<(), AppError> {
	extract_with_limit(bytes, dest, MAX_EXTRACTED_BYTES)
}

fn extract_with_limit(bytes: &[u8], dest: &Path, max_extracted_bytes: u64) -> Result<(), AppError> {
	let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
		.map_err(|err| AppError::BadRequest(format!("Invalid zip file: {err}")))?;
	let mut extracted_bytes: u64 = 0;
	for i in 0..archive.len() {
		let mut entry = archive.by_index(i).map_err(|err| AppError::BadRequest(format!("Invalid zip entry: {err}")))?;
		let Some(relative_path) = entry.enclosed_name() else {
			return Err(AppError::BadRequest(format!("Zip entry has an unsafe path: {}", entry.name())));
		};
		let out_path = dest.join(relative_path);
		if entry.is_dir() {
			fs::create_dir_all(&out_path)?;
			continue;
		}
		if let Some(parent) = out_path.parent() {
			fs::create_dir_all(parent)?;
		}
		extracted_bytes = extracted_bytes.saturating_add(entry.size());
		if extracted_bytes > max_extracted_bytes {
			return Err(AppError::BadRequest(format!(
				"Zip contents exceed the {max_extracted_bytes}-byte extraction limit"
			)));
		}
		let mut out_file = fs::File::create(&out_path)?;
		io::copy(&mut entry, &mut out_file)?;
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use std::io::Write as _;

	use super::*;

	/// Build an in-memory zip. `raw_name` is written as-is (no sanitization), so
	/// tests can construct zip-slip payloads that a well-behaved writer wouldn't produce.
	fn build_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
		let mut buf = Vec::new();
		{
			let mut writer = zip::ZipWriter::new(io::Cursor::new(&mut buf));
			let options = zip::write::SimpleFileOptions::default();
			for (name, contents) in entries {
				writer.start_file(*name, options).unwrap();
				writer.write_all(contents).unwrap();
			}
			writer.finish().unwrap();
		}
		buf
	}

	#[test]
	fn extracts_normal_entries() {
		let zip = build_zip(&[("src/main.rs", b"fn main() {}")]);
		let dest = tempfile::tempdir().unwrap();
		extract(&zip, dest.path()).unwrap();
		assert_eq!(fs::read_to_string(dest.path().join("src/main.rs")).unwrap(), "fn main() {}");
	}

	#[test]
	fn rejects_zip_slip_entries() {
		let zip = build_zip(&[("../../evil.txt", b"pwned")]);
		let dest = tempfile::tempdir().unwrap();
		let err = extract(&zip, dest.path()).unwrap_err();
		assert!(matches!(err, AppError::BadRequest(msg) if msg.contains("unsafe path")));
	}

	#[test]
	fn rejects_invalid_zip_data() {
		let dest = tempfile::tempdir().unwrap();
		let err = extract(b"not a zip file", dest.path()).unwrap_err();
		assert!(matches!(err, AppError::BadRequest(_)));
	}

	#[test]
	fn rejects_extraction_over_the_configured_limit() {
		let zip = build_zip(&[("big.bin", &[0u8; 1024])]);
		let dest = tempfile::tempdir().unwrap();
		let err = extract_with_limit(&zip, dest.path(), 100).unwrap_err();
		assert!(matches!(err, AppError::BadRequest(msg) if msg.contains("extraction limit")));
	}
}
