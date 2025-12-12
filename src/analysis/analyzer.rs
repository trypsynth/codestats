use std::{
	mem,
	path::{Path, PathBuf},
	sync::{
		Arc, Mutex, PoisonError,
		atomic::{AtomicU64, Ordering},
	},
};

use anyhow::Result;
use ignore::WalkBuilder;

use super::{file_processor, stats::AnalysisResults};
use crate::config::AnalyzerConfig;

struct LocalAggregator {
	sink: Arc<Mutex<Vec<AnalysisResults>>>,
	local: AnalysisResults,
}

impl Drop for LocalAggregator {
	fn drop(&mut self) {
		let mut sink = self.sink.lock().unwrap_or_else(PoisonError::into_inner);
		sink.push(mem::take(&mut self.local));
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
	pub fn new(path: &Path, config: AnalyzerConfig) -> Self {
		Self { root: path.to_path_buf(), config }
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
		let error_counter = Arc::new(AtomicU64::new(0));
		let verbose = self.config.verbose;
		let collect_details = self.config.collect_file_details;
		let aggregates = Arc::new(Mutex::new(Vec::new()));
		let aggregates_for_walk = Arc::clone(&aggregates);
		let error_counter_for_walk = Arc::clone(&error_counter);
		WalkBuilder::new(&self.root)
			.follow_links(self.config.follow_symlinks)
			.ignore(self.config.respect_gitignore)
			.git_ignore(self.config.respect_gitignore)
			.require_git(false)
			.hidden(!self.config.include_hidden)
			.build_parallel()
			.run(move || {
				let mut aggregator =
					LocalAggregator { sink: Arc::clone(&aggregates_for_walk), local: AnalysisResults::default() };
				let error_counter = Arc::clone(&error_counter_for_walk);
				Box::new(move |entry_result| {
					match entry_result {
						Ok(entry) if entry.file_type().is_some_and(|ft| ft.is_file()) => {
							// Consider all UTF-8 file names; language detection will skip binaries/unknowns.
							let should_consider = entry.file_name().to_str().is_some();
							if should_consider {
								if let Err(err) =
									file_processor::process_file(entry.path(), &mut aggregator.local, collect_details)
								{
									if verbose {
										eprintln!("Failed to process {}: {err}", entry.path().display());
									}
									error_counter.fetch_add(1, Ordering::Relaxed);
								}
							}
						}
						Err(err) => {
							if verbose {
								eprintln!("Walker error: {err}");
							}
							error_counter.fetch_add(1, Ordering::Relaxed);
						}
						_ => {}
					}
					ignore::WalkState::Continue
				})
			});
		let partials = Arc::try_unwrap(aggregates)
			.map_err(|_| anyhow::anyhow!("Failed to unwrap aggregates Arc - walker still holds references"))?
			.into_inner()
			.unwrap_or_else(PoisonError::into_inner);
		let results = partials.into_iter().fold(AnalysisResults::default(), |mut acc, mut local| {
			acc.merge(mem::take(&mut local));
			acc
		});
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
