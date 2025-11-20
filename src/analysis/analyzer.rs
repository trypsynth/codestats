use std::{
	fs::File,
	io::{BufRead, BufReader, Seek, SeekFrom},
	path::{Path, PathBuf},
	sync::{Arc, Mutex, PoisonError},
};

use anyhow::{Context, Result};
use ignore::WalkBuilder;

use super::{
	line_classifier::{self, CommentState, LineType},
	stats::{AnalysisResults, FileContribution, FileStats},
};
use crate::langs;

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
		let results = Arc::new(Mutex::new(AnalysisResults::default()));
		let verbose = self.config.verbose;
		let collect_details = self.config.detail_level.collect_file_details();
		WalkBuilder::new(&self.root)
			.follow_links(self.config.traversal.follow_symlinks)
			.ignore(self.config.traversal.respect_gitignore)
			.git_ignore(self.config.traversal.respect_gitignore)
			.hidden(!self.config.traversal.include_hidden)
			.build_parallel()
			.run(|| {
				let results = Arc::clone(&results);
				let detail_collection = collect_details;
				Box::new(move |entry_result| {
					match entry_result {
						Ok(entry) if entry.file_type().is_some_and(|ft| ft.is_file()) => {
							if let Err(e) = Self::process_file(entry.path(), &results, detail_collection) {
								if verbose {
									eprintln!("Error processing file {}: {e}", entry.path().display());
								}
							}
						}
						Err(e) if verbose => {
							eprintln!("Error walking directory: {e}");
						}
						_ => {}
					}
					ignore::WalkState::Continue
				})
			});
		let mut results = Arc::try_unwrap(results)
			.map_err(|_| anyhow::anyhow!("Failed to unwrap Arc - parallel walker still holds references"))?
			.into_inner()
			.unwrap_or_else(PoisonError::into_inner);
		results.finalize();
		Ok(results)
	}

	fn process_file(file_path: &Path, results: &Arc<Mutex<AnalysisResults>>, collect_details: bool) -> Result<()> {
		let filename = file_path.file_name().and_then(|name| name.to_str()).context("Invalid UTF-8 in file name")?;
		let metadata =
			file_path.metadata().with_context(|| format!("Failed to read metadata for {}", file_path.display()))?;
		let file_size = metadata.len();
		let file = File::open(file_path).with_context(|| format!("Failed to open file {}", file_path.display()))?;
		let mut reader = BufReader::new(file);
		let sample_content = Self::read_detection_sample(&mut reader)?;
		let sample_ref = if sample_content.is_empty() { None } else { Some(sample_content) };
		let Some(language) = langs::detect_language_info(filename, sample_ref.as_deref()) else {
			return Ok(());
		};
		let lang_info = Some(language);
		let mut total_lines = 0;
		let mut code_lines = 0;
		let mut comment_lines = 0;
		let mut blank_lines = 0;
		let mut shebang_lines = 0;
		let mut comment_state = CommentState::new();
		let mut buffer = Vec::new();
		let mut is_first_line = true;
		loop {
			buffer.clear();
			let bytes_read = reader.read_until(b'\n', &mut buffer)?;
			if bytes_read == 0 {
				break;
			}
			let line = String::from_utf8_lossy(&buffer);
			let line_type = line_classifier::classify_line(line.as_ref(), lang_info, &mut comment_state, is_first_line);
			match line_type {
				LineType::Code => code_lines += 1,
				LineType::Comment => comment_lines += 1,
				LineType::Blank => blank_lines += 1,
				LineType::Shebang => shebang_lines += 1,
			}
			total_lines += 1;
			is_first_line = false;
		}
		let contribution =
			FileContribution::new(total_lines, code_lines, comment_lines, blank_lines, shebang_lines, file_size);
		let file_stats = if collect_details {
			let file_path_str = file_path.display().to_string();
			Some(FileStats::new(
				file_path_str,
				total_lines,
				code_lines,
				comment_lines,
				blank_lines,
				shebang_lines,
				file_size,
			))
		} else {
			None
		};
		results.lock().unwrap_or_else(PoisonError::into_inner).add_file_stats(language.name, contribution, file_stats);
		Ok(())
	}

	fn read_detection_sample(reader: &mut BufReader<File>) -> Result<String> {
		const SAMPLE_BYTES: usize = 16 * 1024;
		let mut sample_bytes = Vec::with_capacity(SAMPLE_BYTES);
		while sample_bytes.len() < SAMPLE_BYTES {
			let buffer = reader.fill_buf()?;
			if buffer.is_empty() {
				break;
			}
			let take = SAMPLE_BYTES.saturating_sub(sample_bytes.len()).min(buffer.len());
			sample_bytes.extend_from_slice(&buffer[..take]);
			reader.consume(take);
		}
		reader.seek(SeekFrom::Start(0)).context("Failed to rewind file for analysis")?;
		Ok(String::from_utf8_lossy(&sample_bytes).into_owned())
	}
}
