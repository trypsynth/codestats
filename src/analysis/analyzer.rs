use std::{
	mem,
	path::PathBuf,
	sync::{
		Arc, Mutex, PoisonError,
		atomic::{AtomicU64, Ordering},
	},
};

use anyhow::Result;
use ignore::WalkBuilder;

use super::{config::AnalyzerConfig, file_processor, stats::AnalysisResults};
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
							if let Err(e) =
								file_processor::process_file(entry.path(), &mut aggregator.local, detail_collection)
							{
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
}
