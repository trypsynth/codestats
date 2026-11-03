//! Counting a list of paths, as `cs -` reads them from standard input: a file is counted as it's named (a link to one as the file it links to), and a directory is walked.

use std::{
	collections::{HashMap, HashSet},
	path::{Path, PathBuf},
	sync::atomic::{AtomicUsize, Ordering},
	thread,
};

use anyhow::Result;

use super::{
	analyzer::CodeAnalyzer,
	attributes::{Attributes, DirCache},
	git::{PathFilter, path_from_git},
	pipeline::{self, FileFilter},
	stats::AnalysisResults,
};
use crate::{config::AnalyzerConfig, display::Verbosity};

/// How many listed paths a worker takes at a time.
const BATCH: usize = 256;

/// The paths in a list, one per line or, if it holds a NUL (as `git ls-files -z` writes), one per NUL-terminated entry. Empty entries and repeats are left out.
pub fn parse_list(list: &[u8]) -> Vec<PathBuf> {
	let separator = if list.contains(&0) { 0 } else { b'\n' };
	let mut seen = HashSet::new();
	list.split(|&b| b == separator)
		.map(|entry| if separator == b'\n' { entry.strip_suffix(b"\r").unwrap_or(entry) } else { entry })
		.filter(|entry| !entry.is_empty() && seen.insert(*entry))
		.map(path_from_git)
		.collect()
}

/// Count the files in `paths` as they're named, and walk the directories among them, with the filters a walk applies except `.gitignore`, since the list chose the files.
///
/// # Errors
///
/// Returns an error if an `--exclude` pattern is invalid, or walking a listed directory fails.
pub fn count_listed(paths: &[PathBuf], config: &AnalyzerConfig) -> Result<AnalysisResults> {
	let analysis = &config.analysis;
	let filter = PathFilter::new(Path::new("."), analysis, false)?;
	let attributes = analysis.respect_gitattributes.then(|| Attributes::on_disk(Path::new("")));
	let languages = FileFilter {
		include: &analysis.include_languages,
		exclude: &analysis.exclude_languages,
		include_generated: analysis.include_generated,
		attributes: attributes.as_ref(),
	};
	let verbose = analysis.verbosity == Verbosity::Verbose;
	let next = AtomicUsize::new(0);
	let threads = thread::available_parallelism().map_or(4, usize::from).min(paths.len().div_ceil(BATCH)).max(1);
	let partials: Vec<(AnalysisResults, Vec<PathBuf>, u64)> = thread::scope(|scope| {
		let workers: Vec<_> = (0..threads)
			.map(|_| {
				scope.spawn(|| {
					let mut results = AnalysisResults::default();
					let mut dirs = Vec::new();
					let mut skipped = 0;
					let mut cache = DirCache::default();
					let mut excluded_dirs = HashMap::new();
					loop {
						let start = next.fetch_add(BATCH, Ordering::Relaxed);
						if start >= paths.len() {
							break;
						}
						for path in &paths[start..paths.len().min(start + BATCH)] {
							if !filter.allows(path, &mut excluded_dirs) {
								continue;
							}
							let counted = pipeline::process_file(
								path,
								Path::new(""),
								&mut results,
								config.collect_file_details,
								languages,
								&mut cache,
							);
							// A directory only shows itself by failing to be read as a file, which saves a `stat` for every listed file.
							if let Err(err) = counted {
								if path.is_dir() {
									// A link to a directory, which `git ls-files` lists among the files, is only followed with `--symlinks`, as in a walk.
									if analysis.follow_symlinks || !path.is_symlink() {
										dirs.push(path.clone());
									}
								} else {
									if verbose {
										eprintln!("Failed to process {}: {err}", path.display());
									}
									skipped += 1;
								}
							}
						}
					}
					(results, dirs, skipped)
				})
			})
			.collect();
		workers.into_iter().map(|worker| worker.join().expect("counting thread panicked")).collect()
	});
	let mut all = AnalysisResults::with_language_capacity();
	let mut skipped = 0;
	let mut dirs = Vec::new();
	for (results, more_dirs, more_skipped) in partials {
		all.merge(results);
		dirs.extend(more_dirs);
		skipped += more_skipped;
	}
	if skipped > 0 {
		eprintln!(
			"Skipped {skipped} listed files due to errors{}.",
			if verbose { "" } else { " (re-run with --verbose for details)" }
		);
	}
	all.set_skipped_entries(all.skipped_entries() + skipped);
	dirs.sort_unstable();
	for dir in &dirs {
		all.merge(CodeAnalyzer::new(dir, config.clone()).showing_root().analyze()?);
	}
	Ok(all)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn lists_split_on_lines_or_nuls() {
		assert_eq!(
			parse_list(b"src/a.rs\r\nsrc/b.rs\n\nsrc/a.rs\n"),
			[PathBuf::from("src/a.rs"), PathBuf::from("src/b.rs")]
		);
		assert_eq!(parse_list(b"a b.rs\0c.rs\0"), [PathBuf::from("a b.rs"), PathBuf::from("c.rs")]);
		assert_eq!(parse_list(b""), Vec::<PathBuf>::new());
	}
}
