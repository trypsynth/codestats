use std::{
	fs::{self, File},
	io::{BufRead, BufReader},
	path::{Path, PathBuf},
	sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use bitflags::bitflags;
use ignore::WalkBuilder;

use super::{
	line_classifier::{self, CommentState, LineType},
	stats::{AnalysisResults, FileStats},
};
use crate::langs;

bitflags! {
	/// Configuration flags for analysis behavior
	#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
	pub struct AnalysisFlags: u8 {
		/// Enable verbose output
		const VERBOSE = 1 << 0;
		/// Respect .gitignore files (enabled by default)
		const RESPECT_GITIGNORE = 1 << 1;
		/// Include hidden files and directories
		const INCLUDE_HIDDEN = 1 << 2;
		/// Follow symbolic links
		const FOLLOW_SYMLINKS = 1 << 3;
	}
}

impl Default for AnalysisFlags {
	fn default() -> Self {
		Self::RESPECT_GITIGNORE
	}
}

/// Configuration options for code analysis
#[derive(Debug, Clone)]
pub struct AnalysisOptions {
	path: PathBuf,
	flags: AnalysisFlags,
}

impl AnalysisOptions {
	/// Create new analysis options with default flags
	pub fn new(path: impl Into<PathBuf>) -> Self {
		Self { path: path.into(), flags: AnalysisFlags::default() }
	}

	/// Create new analysis options with custom flags
	pub fn with_flags(path: impl Into<PathBuf>, flags: AnalysisFlags) -> Self {
		Self { path: path.into(), flags }
	}

	/// Enable or disable verbose output
	#[must_use]
	pub const fn verbose(mut self, verbose: bool) -> Self {
		if verbose {
			self.flags = self.flags.union(AnalysisFlags::VERBOSE);
		} else {
			self.flags = self.flags.difference(AnalysisFlags::VERBOSE);
		}
		self
	}

	/// Enable or disable respecting .gitignore files
	#[must_use]
	pub const fn respect_gitignore(mut self, respect: bool) -> Self {
		if respect {
			self.flags = self.flags.union(AnalysisFlags::RESPECT_GITIGNORE);
		} else {
			self.flags = self.flags.difference(AnalysisFlags::RESPECT_GITIGNORE);
		}
		self
	}

	/// Enable or disable including hidden files
	#[must_use]
	pub const fn include_hidden(mut self, include: bool) -> Self {
		if include {
			self.flags = self.flags.union(AnalysisFlags::INCLUDE_HIDDEN);
		} else {
			self.flags = self.flags.difference(AnalysisFlags::INCLUDE_HIDDEN);
		}
		self
	}

	/// Enable or disable following symbolic links
	#[must_use]
	pub const fn follow_symlinks(mut self, follow: bool) -> Self {
		if follow {
			self.flags = self.flags.union(AnalysisFlags::FOLLOW_SYMLINKS);
		} else {
			self.flags = self.flags.difference(AnalysisFlags::FOLLOW_SYMLINKS);
		}
		self
	}

	/// Get the path to analyze
	#[must_use]
	pub fn path(&self) -> &Path {
		&self.path
	}

	/// Check if verbose output is enabled
	#[must_use]
	pub const fn is_verbose(&self) -> bool {
		self.flags.contains(AnalysisFlags::VERBOSE)
	}

	/// Get the analysis flags
	#[must_use]
	pub const fn flags(&self) -> AnalysisFlags {
		self.flags
	}
}

pub struct CodeAnalyzer {
	options: AnalysisOptions,
}

impl CodeAnalyzer {
	#[must_use]
	pub const fn new(options: AnalysisOptions) -> Self {
		Self { options }
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
		let flags = self.options.flags();
		if flags.contains(AnalysisFlags::VERBOSE) {
			println!("Analyzing directory {}", self.options.path.display());
		}
		let results = Arc::new(Mutex::new(AnalysisResults::default()));
		let verbose = flags.contains(AnalysisFlags::VERBOSE);
		WalkBuilder::new(&self.options.path)
			.follow_links(flags.contains(AnalysisFlags::FOLLOW_SYMLINKS))
			.ignore(flags.contains(AnalysisFlags::RESPECT_GITIGNORE))
			.git_ignore(flags.contains(AnalysisFlags::RESPECT_GITIGNORE))
			.hidden(!flags.contains(AnalysisFlags::INCLUDE_HIDDEN))
			.build_parallel()
			.run(|| {
				let results = Arc::clone(&results);
				Box::new(move |entry_result| {
					match entry_result {
						Ok(entry) if entry.file_type().is_some_and(|ft| ft.is_file()) => {
							if let Err(e) = Self::process_file(entry.path(), &results) {
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
		Ok(Arc::try_unwrap(results).unwrap().into_inner().unwrap())
	}

	fn process_file(file_path: &Path, results: &Arc<Mutex<AnalysisResults>>) -> Result<()> {
		let filename = file_path.file_name().and_then(|name| name.to_str()).context("Invalid UTF-8 in file name")?;
		let Some(language) = langs::detect_language(filename) else {
			return Ok(());
		};
		let file_size = fs::metadata(file_path)
			.with_context(|| format!("Failed to retrieve metadata for {}", file_path.display()))?
			.len();
		let (total_lines, code_lines, comment_lines, blank_lines, shebang_lines) =
			Self::analyze_file_lines(file_path, language)?;
		let file_path_str = file_path.display().to_string();
		let file_stats = FileStats::new(
			file_path_str,
			total_lines,
			code_lines,
			comment_lines,
			blank_lines,
			shebang_lines,
			file_size,
		);
		results.lock().unwrap().add_file_stats(language.to_string(), file_stats);
		Ok(())
	}

	fn analyze_file_lines(file_path: &Path, language: &str) -> Result<(u64, u64, u64, u64, u64)> {
		let file = File::open(file_path).with_context(|| format!("Failed to open file {}", file_path.display()))?;
		let reader = BufReader::new(file);
		let lang_info = langs::get_language_info(language);
		let mut line_counts = (0, 0, 0, 0, 0); // total, code, comment, blank, shebang
		let mut comment_state = CommentState::new();
		let mut line_number = 0;
		for line_result in reader.lines() {
			let line = line_result.with_context(|| format!("Failed to read line from {}", file_path.display()))?;
			line_counts.0 += 1; // total_lines
			line_number += 1;
			let line_type = line_classifier::classify_line(&line, lang_info, &mut comment_state, line_number == 1);
			match line_type {
				LineType::Code => line_counts.1 += 1,
				LineType::Comment => line_counts.2 += 1,
				LineType::Blank => line_counts.3 += 1,
				LineType::Shebang => line_counts.4 += 1,
			}
		}
		Ok(line_counts)
	}
}
