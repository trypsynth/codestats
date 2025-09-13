use std::{
	fs::{self, File},
	io::{BufRead, BufReader},
	path::{Path, PathBuf},
	sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use ignore::WalkBuilder;

use super::{
	line_classifier::{self, CommentState, LineType},
	stats::{AnalysisResults, FileStats},
};
use crate::language;

#[derive(Debug, Clone)]
pub struct AnalysisOptions {
	path: PathBuf,
	verbose: bool,
	respect_gitignore: bool,
	include_hidden: bool,
	follow_symlinks: bool,
}

impl AnalysisOptions {
	pub fn new(path: impl Into<PathBuf>) -> Self {
		Self {
			path: path.into(),
			verbose: false,
			respect_gitignore: true,
			include_hidden: false,
			follow_symlinks: false,
		}
	}

	#[must_use]
	pub const fn verbose(mut self, verbose: bool) -> Self {
		self.verbose = verbose;
		self
	}

	#[must_use]
	pub const fn respect_gitignore(mut self, respect: bool) -> Self {
		self.respect_gitignore = respect;
		self
	}

	#[must_use]
	pub const fn include_hidden(mut self, include: bool) -> Self {
		self.include_hidden = include;
		self
	}

	#[must_use]
	pub const fn follow_symlinks(mut self, follow: bool) -> Self {
		self.follow_symlinks = follow;
		self
	}

	#[must_use]
	pub fn path(&self) -> &Path {
		&self.path
	}

	#[must_use]
	pub const fn is_verbose(&self) -> bool {
		self.verbose
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

	pub fn analyze(&self) -> Result<AnalysisResults> {
		if self.options.verbose {
			println!("Analyzing directory {}", self.options.path.display());
		}
		let results = Arc::new(Mutex::new(AnalysisResults::default()));
		let verbose = self.options.verbose;
		WalkBuilder::new(&self.options.path)
			.follow_links(self.options.follow_symlinks)
			.ignore(self.options.respect_gitignore)
			.git_ignore(self.options.respect_gitignore)
			.hidden(!self.options.include_hidden)
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
		let Some(language) = language::detect_language(filename) else {
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
		let lang_info = language::get_language_info(language);
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
