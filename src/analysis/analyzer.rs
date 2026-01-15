use std::{
	path::{Path, PathBuf},
	sync::{
		Arc, Mutex, PoisonError,
		atomic::{AtomicU64, Ordering},
	},
};

use anyhow::Result;
use ignore::{WalkBuilder, overrides::OverrideBuilder};

use super::{pipeline, stats::AnalysisResults};
use crate::config::AnalyzerConfig;

/// Thread-local accumulator for parallel file analysis.
///
/// This struct implements a clever pattern to reduce lock contention during parallel processing:
/// - Each worker thread maintains its own local `AnalysisResults` instance.
/// - Workers accumulate statistics without any synchronization overhead.
/// - On `Drop`, the local results are merged into the shared sink in one batch.
///
/// This dramatically reduces mutex contention compared to locking on every file processed, while ensuring all results are properly collected even if a worker panics.
struct LocalAggregator {
	sink: Arc<Mutex<Vec<AnalysisResults>>>,
	local: AnalysisResults,
}

impl Drop for LocalAggregator {
	fn drop(&mut self) {
		let local = std::mem::take(&mut self.local);
		let mut sink = self.sink.lock().unwrap_or_else(PoisonError::into_inner);
		sink.push(local);
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
		let verbose = self.config.analysis.verbose;
		let collect_details = self.config.collect_file_details;
		let include_languages = self.config.analysis.include_languages.clone();
		let exclude_languages = self.config.analysis.exclude_languages.clone();
		let aggregates = Arc::new(Mutex::new(Vec::new()));
		let aggregates_for_walk = Arc::clone(&aggregates);
		let error_counter_for_walk = Arc::clone(&error_counter);
		let mut builder = WalkBuilder::new(&self.root);
		builder
			.follow_links(self.config.analysis.follow_symlinks)
			.ignore(self.config.analysis.respect_gitignore)
			.git_ignore(self.config.analysis.respect_gitignore)
			.git_global(self.config.analysis.respect_gitignore)
			.git_exclude(self.config.analysis.respect_gitignore)
			.require_git(false)
			.hidden(!self.config.analysis.include_hidden);
		if !self.config.analysis.exclude_patterns.is_empty() {
			let mut override_builder = OverrideBuilder::new(&self.root);
			for pattern in &self.config.analysis.exclude_patterns {
				// OverrideBuilder treats patterns without '!' as include rules, so we invert to enforce exclusion.
				let owned;
				let glob = if pattern.starts_with('!') {
					pattern.as_str()
				} else {
					owned = format!("!{pattern}");
					owned.as_str()
				};
				override_builder.add(glob)?;
			}
			builder.overrides(override_builder.build()?);
		}
		builder.build_parallel().run(move || {
			let mut aggregator =
				LocalAggregator { sink: Arc::clone(&aggregates_for_walk), local: AnalysisResults::default() };
			let error_counter = Arc::clone(&error_counter_for_walk);
			let include_languages = include_languages.clone();
			let exclude_languages = exclude_languages.clone();
			Box::new(move |entry_result| {
				match entry_result {
					Ok(entry) if entry.file_type().is_some_and(|ft| ft.is_file()) => {
						if let Err(err) = pipeline::process_file(
							entry.path(),
							&mut aggregator.local,
							collect_details,
							&include_languages,
							&exclude_languages,
						) {
							if verbose {
								eprintln!("Failed to process {}: {err}", entry.path().display());
							}
							error_counter.fetch_add(1, Ordering::Relaxed);
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
		let mut results = partials.into_iter().fold(AnalysisResults::with_language_capacity(), |mut acc, local| {
			acc.merge(local);
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
		results.set_skipped_entries(skipped);
		Ok(results)
	}
}
