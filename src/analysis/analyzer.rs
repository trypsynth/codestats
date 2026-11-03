use std::{
	path::{Path, PathBuf},
	sync::{
		Mutex, PoisonError,
		atomic::{AtomicU64, Ordering},
	},
	thread,
};

use anyhow::Result;

use super::{
	attributes::{Attributes, DirCache},
	git,
	pipeline::{self, FileFilter},
	stats::AnalysisResults,
	walk::{self, WalkOptions},
};
use crate::{config::AnalyzerConfig, display::Verbosity};

/// A worker thread's own results, handed to the shared sink once when the worker finishes, so threads don't contend on a lock per file.
struct LocalAggregator<'a> {
	sink: &'a Mutex<Vec<AnalysisResults>>,
	local: AnalysisResults,
}

impl Drop for LocalAggregator<'_> {
	fn drop(&mut self) {
		let local = std::mem::take(&mut self.local);
		let mut sink = self.sink.lock().unwrap_or_else(PoisonError::into_inner);
		sink.push(local);
	}
}

/// Walks source files within a directory tree and produces aggregated statistics.
pub struct CodeAnalyzer {
	root: PathBuf,
	/// What files' paths are shown relative to: the root, unless several roots are counted together.
	display_root: PathBuf,
	config: AnalyzerConfig,
}

impl CodeAnalyzer {
	/// Create a new analyzer rooted at `path`.
	#[must_use]
	pub fn new(path: &Path, config: AnalyzerConfig) -> Self {
		Self { root: path.to_path_buf(), display_root: path.to_path_buf(), config }
	}

	/// Show files' paths as they're reached from the root, root included (`src/main.rs` rather than `main.rs`), as when several roots are counted together.
	#[must_use]
	pub fn showing_root(mut self) -> Self {
		self.display_root = PathBuf::new();
		self
	}

	/// Analyze the configured path for code statistics
	///
	/// # Errors
	///
	/// Returns an error if:
	/// - The path cannot be accessed
	/// - File I/O operations fail during analysis
	/// - UTF-8 decoding fails for file contents
	pub fn analyze(&self) -> Result<AnalysisResults> {
		if self.config.analysis.git_tracked_only {
			let snapshot = git::Snapshot::WorkingTree { submodules: true };
			return git::count_snapshot(&self.root, &snapshot, &self.config.analysis, self.config.collect_file_details)
				.map_err(|err| anyhow::anyhow!("--git: {err}"));
		}
		let error_counter = AtomicU64::new(0);
		let verbosity = self.config.analysis.verbosity;
		let collect_details = self.config.collect_file_details;
		let attributes = self.config.analysis.respect_gitattributes.then(|| Attributes::on_disk(&self.root));
		let root = &self.display_root;
		let aggregates = Mutex::new(Vec::new());
		let options = WalkOptions {
			follow_links: self.config.analysis.follow_symlinks,
			skip_hidden: !self.config.analysis.include_hidden,
			max_depth: self.config.analysis.max_depth,
			respect_ignores: self.config.analysis.respect_gitignore,
			overrides: git::exclusions(&self.root, &self.config.analysis)?,
			threads: thread::available_parallelism().map_or(1, usize::from),
		};
		let filter = FileFilter {
			include: &self.config.analysis.include_languages,
			exclude: &self.config.analysis.exclude_languages,
			include_generated: self.config.analysis.include_generated,
			attributes: attributes.as_ref(),
			extensions: &self.config.analysis.include_extensions,
		};
		walk::walk(&self.root, options, &|| {
			let mut aggregator = LocalAggregator { sink: &aggregates, local: AnalysisResults::default() };
			let mut cache = DirCache::default();
			let error_counter = &error_counter;
			move |entry_result: Result<walk::Entry, ignore::Error>| match entry_result {
				Ok(entry) => {
					if let Err(err) = pipeline::process_file(
						entry.path(),
						root,
						&mut aggregator.local,
						collect_details,
						filter,
						&mut cache,
					) {
						if verbosity == Verbosity::Verbose {
							eprintln!("Failed to process {}: {err}", entry.path().display());
						}
						error_counter.fetch_add(1, Ordering::Relaxed);
					}
				}
				Err(err) => {
					if verbosity == Verbosity::Verbose {
						eprintln!("Walker error: {err}");
					}
					error_counter.fetch_add(1, Ordering::Relaxed);
				}
			}
		});
		let partials = aggregates.into_inner().unwrap_or_else(PoisonError::into_inner);
		let mut results = partials.into_iter().fold(AnalysisResults::with_language_capacity(), |mut acc, local| {
			acc.merge(local);
			acc
		});
		let skipped = error_counter.load(Ordering::Relaxed);
		if skipped > 0 {
			if verbosity == Verbosity::Verbose {
				eprintln!("Skipped {skipped} entries due to errors.");
			} else {
				eprintln!("Skipped {skipped} entries due to errors (re-run with --verbose for details).");
			}
		}
		results.set_skipped_entries(skipped);
		Ok(results)
	}
}
