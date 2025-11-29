use std::{
	fs::File,
	io::{BufRead, BufReader},
	mem,
	path::{Path, PathBuf},
	str,
	sync::{
		Arc, Mutex, PoisonError,
		atomic::{AtomicU64, Ordering},
	},
};

use anyhow::{Context, Result};
use ignore::WalkBuilder;
use memmap2::Mmap;

use super::{
	line_classifier::{self, CommentState, LineType},
	stats::{AnalysisResults, FileContribution, FileStats},
};
use crate::langs;

struct LocalAggregator {
	shared: Arc<Mutex<AnalysisResults>>,
	local: AnalysisResults,
}

impl Drop for LocalAggregator {
	fn drop(&mut self) {
		let mut shared = self.shared.lock().unwrap_or_else(PoisonError::into_inner);
		shared.merge(mem::take(&mut self.local));
	}
}

struct LineCounts {
	total: u64,
	code: u64,
	comment: u64,
	blank: u64,
	shebang: u64,
}

impl LineCounts {
	const fn new() -> Self {
		Self { total: 0, code: 0, comment: 0, blank: 0, shebang: 0 }
	}

	fn classify_and_count(
		&mut self,
		line_bytes: &[u8],
		lang_info: Option<&langs::Language>,
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

/// Configuration that controls how [`CodeAnalyzer`] traverses the filesystem and how much information it gathers.
#[derive(Clone, Debug, Default)]
pub struct AnalyzerConfig {
	/// Emit additional progress messages and per-file diagnostics.
	pub verbose: bool,
	/// Controls what should be considered while walking directories.
	pub traversal: TraversalOptions,
	/// Select whether only aggregated totals or per-file data should be collected.
	pub detail_level: DetailLevel,
}

/// Options that influence how [`CodeAnalyzer`] traverses directories.
#[derive(Clone, Copy, Debug)]
pub struct TraversalOptions {
	/// Respect `.gitignore` files while walking.
	pub respect_gitignore: bool,
	/// Include hidden files and directories.
	pub include_hidden: bool,
	/// Follow symbolic links discovered during traversal.
	pub follow_symlinks: bool,
}

impl Default for TraversalOptions {
	fn default() -> Self {
		Self { respect_gitignore: true, include_hidden: false, follow_symlinks: false }
	}
}

/// Controls how much information is tracked for each file that matches the filters.
#[derive(Clone, Copy, Debug, Default)]
pub enum DetailLevel {
	/// Collect only aggregated totals per language.
	#[default]
	Summary,
	/// Collect aggregated totals plus detailed statistics for every file.
	PerFile,
}

impl DetailLevel {
	#[must_use]
	const fn collect_file_details(self) -> bool {
		matches!(self, Self::PerFile)
	}
}

/// Walks source files within a directory tree and produces aggregated statistics.
pub struct CodeAnalyzer {
	root: PathBuf,
	config: AnalyzerConfig,
}

impl CodeAnalyzer {
	/// Create a new analyzer rooted at `path`.
	#[must_use]
	pub fn new(path: impl Into<PathBuf>, config: AnalyzerConfig) -> Self {
		Self { root: path.into(), config }
	}

	/// Analyze the configured path for code statistics
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The path cannot be accessed
	/// - File I/O operations fail during analysis
	/// - UTF-8 decoding fails for file contents
	///
	/// # Panics
	///
	/// May panic if the internal Arc or Mutex operations fail unexpectedly,
	/// which should hopefully never happen.
	pub fn analyze(&self) -> Result<AnalysisResults> {
		if self.config.verbose {
			println!("Analyzing directory {}", self.root.display());
		}
		let results = Arc::new(Mutex::default());
		let shared_results = Arc::clone(&results);
		let error_counter = Arc::new(AtomicU64::new(0));
		let shared_error_counter = Arc::clone(&error_counter);
		let verbose = self.config.verbose;
		let collect_details = self.config.detail_level.collect_file_details();
		let language_globset = langs::language_globset();
		WalkBuilder::new(&self.root)
			.follow_links(self.config.traversal.follow_symlinks)
			.ignore(self.config.traversal.respect_gitignore)
			.git_ignore(self.config.traversal.respect_gitignore)
			.hidden(!self.config.traversal.include_hidden)
			.build_parallel()
			.run(move || {
				let mut aggregator =
					LocalAggregator { shared: Arc::clone(&shared_results), local: AnalysisResults::default() };
				let detail_collection = collect_details;
				let language_globset = language_globset;
				let error_counter = Arc::clone(&shared_error_counter);
				Box::new(move |entry_result| {
					match entry_result {
						Ok(entry) if entry.file_type().is_some_and(|ft| ft.is_file()) => {
							let should_consider = entry
								.file_name()
								.to_str()
								.is_none_or(|name| language_globset.is_match(name) || !name.contains('.'));
							if !should_consider {
								return ignore::WalkState::Continue;
							}
							if let Err(e) = Self::process_file(entry.path(), &mut aggregator.local, detail_collection) {
								error_counter.fetch_add(1, Ordering::Relaxed);
								if verbose {
									eprintln!("Error processing file {}: {e}", entry.path().display());
								}
							}
						}
						Err(e) if verbose => {
							eprintln!("Error walking directory: {e}");
							error_counter.fetch_add(1, Ordering::Relaxed);
						}
						Err(_) => {
							error_counter.fetch_add(1, Ordering::Relaxed);
						}
						_ => {}
					}
					ignore::WalkState::Continue
				})
			});
		let results = Arc::try_unwrap(results)
			.map_err(|_| anyhow::anyhow!("Failed to unwrap Arc - parallel walker still holds references"))?
			.into_inner()
			.unwrap_or_else(PoisonError::into_inner);
		let skipped = error_counter.load(Ordering::Relaxed);
		if skipped > 0 {
			if verbose {
				eprintln!("Skipped {skipped} entries due to errors.");
			} else {
				eprintln!("Skipped {skipped} entries due to errors (re-run with --verbose for details).");
			}
		}
		Ok(results)
	}

	fn process_file(file_path: &Path, results: &mut AnalysisResults, collect_details: bool) -> Result<()> {
		const MMAP_THRESHOLD: u64 = 256 * 1024; // 256KB threshold
		let filename = file_path.file_name().and_then(|name| name.to_str()).context("Invalid UTF-8 in file name")?;
		let metadata =
			file_path.metadata().with_context(|| format!("Failed to read metadata for {}", file_path.display()))?;
		let file_size = metadata.len();
		if file_size == 0 {
			return Ok(());
		}
		if file_size >= MMAP_THRESHOLD {
			Self::process_file_mmap(file_path, filename, file_size, results, collect_details)
		} else {
			Self::process_file_buffered(file_path, filename, file_size, results, collect_details)
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
		if Self::is_probably_binary(&sample_bytes) {
			return Ok(());
		}
		let sample_str = str::from_utf8(&sample_bytes).ok();
		let Some(language) = langs::detect_language_info(filename, sample_str) else { return Ok(()) };
		let mut line_counts = LineCounts::new();
		let mut comment_state = CommentState::new();
		let mut buffer = Vec::with_capacity(1024);
		let mut is_first_line = true;
		loop {
			buffer.clear();
			let bytes_read = reader.read_until(b'\n', &mut buffer)?;
			if bytes_read == 0 {
				break;
			}
			line_counts.classify_and_count(&buffer, Some(language), &mut comment_state, is_first_line);
			is_first_line = false;
		}
		let contribution = FileContribution::new(
			line_counts.total,
			line_counts.code,
			line_counts.comment,
			line_counts.blank,
			line_counts.shebang,
			file_size,
		);
		let file_stats = collect_details.then(|| {
			FileStats::new(
				file_path.display().to_string(),
				line_counts.total,
				line_counts.code,
				line_counts.comment,
				line_counts.blank,
				line_counts.shebang,
				file_size,
			)
		});
		results.add_file_stats(language, contribution, file_stats);
		Ok(())
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
		let mmap = unsafe { Mmap::map(&file) }
			.with_context(|| format!("Failed to memory-map file {}", file_path.display()))?;
		let file_bytes = &mmap[..];
		let sample_size = file_bytes.len().min(4 * 1024);
		let sample_bytes = &file_bytes[..sample_size];
		if Self::is_probably_binary(sample_bytes) {
			return Ok(());
		}
		let sample_str = str::from_utf8(sample_bytes).ok();
		let Some(language) = langs::detect_language_info(filename, sample_str) else { return Ok(()) };
		let mut line_counts = LineCounts::new();
		let mut comment_state = CommentState::new();
		let mut is_first_line = true;
		let mut pos = 0;
		while pos < file_bytes.len() {
			let line_end =
				memchr::memchr(b'\n', &file_bytes[pos..]).map_or(file_bytes.len(), |offset| pos + offset + 1);
			let line_bytes = &file_bytes[pos..line_end];
			line_counts.classify_and_count(line_bytes, Some(language), &mut comment_state, is_first_line);
			is_first_line = false;
			pos = line_end;
		}
		let contribution = FileContribution::new(
			line_counts.total,
			line_counts.code,
			line_counts.comment,
			line_counts.blank,
			line_counts.shebang,
			file_size,
		);
		let file_stats = collect_details.then(|| {
			FileStats::new(
				file_path.display().to_string(),
				line_counts.total,
				line_counts.code,
				line_counts.comment,
				line_counts.blank,
				line_counts.shebang,
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
}
