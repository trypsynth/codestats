use std::{
	borrow::Cow,
	ops::Range,
	path::{Path, PathBuf},
	process::Command,
	sync::{
		Arc, Mutex, PoisonError,
		atomic::{AtomicU64, Ordering},
	},
};

use anyhow::{Context as _, Result, ensure};
use ignore::{WalkBuilder, overrides::OverrideBuilder};

use super::{pipeline, stats::AnalysisResults};
use crate::{config::AnalyzerConfig, display::Verbosity};

/// A worker thread's own results, handed to the shared sink once when the worker finishes, so threads don't contend on a lock per file.
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

/// The files git tracks under an analysis root, for `--git`.
struct TrackedFiles {
	/// The directory git was asked about: the root, or a file root's directory.
	base: PathBuf,
	/// `git ls-files -z` output: paths relative to `base`, `/`-separated.
	listing: Vec<u8>,
	/// Where each path lies in `listing`, sorted by path so lookups are a binary search with no allocation per path.
	paths: Vec<Range<usize>>,
}

impl TrackedFiles {
	fn list(root: &Path) -> Result<Self> {
		let base = if root.is_file() {
			root.parent().unwrap_or_else(|| Path::new("")).to_path_buf()
		} else {
			root.to_path_buf()
		};
		let dir = if base.as_os_str().is_empty() { Path::new(".") } else { &base };
		let output = Command::new("git")
			.arg("-C")
			.arg(dir)
			.args(["ls-files", "-z", "--recurse-submodules"])
			.output()
			.context("--git needs git installed")?;
		ensure!(
			output.status.success(),
			"--git: {}",
			String::from_utf8_lossy(&output.stderr).trim().trim_start_matches("fatal: ")
		);
		let listing = output.stdout;
		let mut paths = Vec::new();
		let mut start = 0;
		for end in memchr::memchr_iter(0, &listing) {
			paths.push(start..end);
			start = end + 1;
		}
		paths.sort_unstable_by(|a, b| listing[a.clone()].cmp(&listing[b.clone()]));
		Ok(Self { base, listing, paths })
	}

	fn contains(&self, path: &Path) -> bool {
		let Ok(relative) = path.strip_prefix(&self.base) else {
			return false;
		};
		let relative = git_path(relative);
		self.paths.binary_search_by(|range| self.listing[range.clone()].cmp(relative.as_ref())).is_ok()
	}
}

/// A relative path spelled as git prints it.
#[cfg(not(windows))]
fn git_path(path: &Path) -> Cow<'_, [u8]> {
	Cow::Borrowed(path.as_os_str().as_encoded_bytes())
}

#[cfg(windows)]
fn git_path(path: &Path) -> Cow<'_, [u8]> {
	Cow::Owned(path.to_string_lossy().replace('\\', "/").into_bytes())
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
	pub fn analyze(&self) -> Result<AnalysisResults> {
		let error_counter = Arc::new(AtomicU64::new(0));
		let verbosity = self.config.analysis.verbosity;
		let collect_details = self.config.collect_file_details;
		let include_languages = self.config.analysis.include_languages.clone();
		let exclude_languages = self.config.analysis.exclude_languages.clone();
		let root = self.root.clone();
		let tracked =
			if self.config.analysis.git_tracked_only { Some(Arc::new(TrackedFiles::list(&self.root)?)) } else { None };
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
			.hidden(!self.config.analysis.include_hidden)
			.max_depth(self.config.analysis.max_depth);
		let needs_overrides =
			!self.config.analysis.exclude_patterns.is_empty() || !self.config.analysis.include_generated;
		if needs_overrides {
			let mut override_builder = OverrideBuilder::new(&self.root);
			if !self.config.analysis.include_generated {
				for pattern in super::generated::PATTERNS {
					override_builder.add(&format!("!{pattern}"))?;
				}
			}
			for pattern in &self.config.analysis.exclude_patterns {
				// OverrideBuilder treats patterns without '!' as include rules, so we invert to enforce exclusion.
				let glob: Cow<'_, str> = if pattern.starts_with('!') {
					Cow::Borrowed(pattern.as_str())
				} else {
					Cow::Owned(format!("!{pattern}"))
				};
				override_builder.add(&glob)?;
			}
			builder.overrides(override_builder.build()?);
		}
		builder.build_parallel().run(move || {
			let mut aggregator =
				LocalAggregator { sink: Arc::clone(&aggregates_for_walk), local: AnalysisResults::default() };
			let error_counter = Arc::clone(&error_counter_for_walk);
			let include_languages = include_languages.clone();
			let exclude_languages = exclude_languages.clone();
			let root = root.clone();
			let tracked = tracked.clone();
			Box::new(move |entry_result| {
				match entry_result {
					Ok(entry)
						if entry.file_type().is_some_and(|ft| ft.is_file())
							&& tracked.as_ref().is_none_or(|tracked| tracked.contains(entry.path())) =>
					{
						if let Err(err) = pipeline::process_file(
							entry.path(),
							&root,
							&mut aggregator.local,
							collect_details,
							&include_languages,
							&exclude_languages,
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
