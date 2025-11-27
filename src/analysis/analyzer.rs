use std::{
	fs::File,
	io::{BufRead, BufReader},
	mem,
	path::{Path, PathBuf},
	str,
	sync::{Arc, Mutex, PoisonError},
};

use anyhow::{Context, Result};
use ignore::WalkBuilder;

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
		let shared_results = Arc::clone(&results);
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
		let results = Arc::try_unwrap(results)
			.map_err(|_| anyhow::anyhow!("Failed to unwrap Arc - parallel walker still holds references"))?
			.into_inner()
			.unwrap_or_else(PoisonError::into_inner);
		Ok(results)
	}

	fn process_file(file_path: &Path, results: &mut AnalysisResults, collect_details: bool) -> Result<()> {
		let filename = file_path.file_name().and_then(|name| name.to_str()).context("Invalid UTF-8 in file name")?;
		let metadata =
			file_path.metadata().with_context(|| format!("Failed to read metadata for {}", file_path.display()))?;
		let file_size = metadata.len();
		let file = File::open(file_path).with_context(|| format!("Failed to open file {}", file_path.display()))?;
		let mut reader = BufReader::new(file);
		let sample_bytes = Self::peek_sample(&mut reader, 4 * 1024)?;
		if Self::is_probably_binary(&sample_bytes) {
			return Ok(());
		}
		let sample_str = str::from_utf8(&sample_bytes).ok();
		let candidates = langs::get_candidates(filename);
		let language = if candidates.is_empty() {
			match langs::detect_language_info(filename, sample_str) {
				Some(lang) => lang,
				None => return Ok(()),
			}
		} else if candidates.len() == 1 {
			candidates[0]
		} else {
			langs::detect_language_info(filename, sample_str).unwrap_or(candidates[0])
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
			let line_type = match str::from_utf8(&buffer) {
				Ok(line) => line_classifier::classify_line(line, lang_info, &mut comment_state, is_first_line),
				Err(_) => {
					let line = String::from_utf8_lossy(&buffer);
					line_classifier::classify_line(line.as_ref(), lang_info, &mut comment_state, is_first_line)
				}
			};
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
		results.add_file_stats(language.name, contribution, file_stats);
		Ok(())
	}

	fn peek_sample(reader: &mut BufReader<File>, max_bytes: usize) -> Result<Vec<u8>> {
		let buffer = reader.fill_buf()?;
		let take = buffer.len().min(max_bytes);
		Ok(buffer[..take].to_vec())
	}

	fn is_probably_binary(sample: &[u8]) -> bool {
		if sample.is_empty() {
			return false;
		}
		let non_text = sample
			.iter()
			.filter(|b| {
				let byte = **b;
				byte == 0 || (byte < 0x09) || (byte > 0x7E && byte < 0xA0)
			})
			.count();
		// Consider binary if more than 10% of the sampled bytes look non-textual.
		non_text * 10 > sample.len()
	}
}
