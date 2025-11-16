use std::{
	fs::File,
	io::{BufRead, BufReader, Seek, SeekFrom},
	path::{Path, PathBuf},
	sync::{Arc, Mutex, PoisonError},
};

use anyhow::{Context, Result};
use bitflags::bitflags;
use ignore::WalkBuilder;

use super::{
	line_classifier::{self, CommentState, LineType},
	stats::{AnalysisResults, FileContribution, FileStats},
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
	collect_file_details: bool,
}

impl AnalysisOptions {
	/// Create new analysis options with default flags
	pub fn new(path: impl Into<PathBuf>) -> Self {
		Self { path: path.into(), flags: AnalysisFlags::default(), collect_file_details: false }
	}

	/// Create new analysis options with custom flags
	pub fn with_flags(path: impl Into<PathBuf>, flags: AnalysisFlags) -> Self {
		Self { path: path.into(), flags, collect_file_details: false }
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

	/// Enable or disable collecting per-file detail
	#[must_use]
	pub const fn include_file_details(mut self, include: bool) -> Self {
		self.collect_file_details = include;
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

	/// Check if file-level statistics should be retained
	#[must_use]
	pub const fn collect_file_details(&self) -> bool {
		self.collect_file_details
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
		let collect_details = self.options.collect_file_details();
		WalkBuilder::new(&self.options.path)
			.follow_links(flags.contains(AnalysisFlags::FOLLOW_SYMLINKS))
			.ignore(flags.contains(AnalysisFlags::RESPECT_GITIGNORE))
			.git_ignore(flags.contains(AnalysisFlags::RESPECT_GITIGNORE))
			.hidden(!flags.contains(AnalysisFlags::INCLUDE_HIDDEN))
			.build_parallel()
			.run(|| {
				let results = Arc::clone(&results);
				let collect_details = collect_details;
				Box::new(move |entry_result| {
					match entry_result {
						Ok(entry) if entry.file_type().is_some_and(|ft| ft.is_file()) => {
							if let Err(e) = Self::process_file(entry.path(), &results, collect_details) {
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
