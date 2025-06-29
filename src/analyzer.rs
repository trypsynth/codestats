use crate::{
	comments::{self, CommentState, LineType},
	langs,
	stats::{FileStats, StatsCollector},
	utils,
};
use anyhow::{Context, Result};
use human_bytes::human_bytes;
use ignore::WalkBuilder;
use std::{
	fs::{self, File},
	io::{BufRead, BufReader},
	path::{Path, PathBuf},
	sync::{Arc, Mutex},
};

pub struct AnalyzerArgs {
	pub path: PathBuf,
	pub verbose: bool,
	pub gitignore: bool,
	pub hidden: bool,
	pub symlinks: bool,
}

/// The heart of codestats, this structure performs all the analysis of a codebase/folder and prints statistics about it.
pub struct CodeAnalyzer {
	args: AnalyzerArgs,
	stats: Arc<Mutex<StatsCollector>>,
}

impl CodeAnalyzer {
	pub fn new(args: AnalyzerArgs) -> Self {
		Self { args, stats: Arc::new(Mutex::new(StatsCollector::default())) }
	}

	pub fn analyze(&mut self) -> Result<()> {
		if self.args.verbose {
			println!("Analyzing directory {}", self.args.path.display());
		}
		let stats = Arc::clone(&self.stats);
		let verbose = self.args.verbose;
		WalkBuilder::new(&self.args.path)
			.follow_links(self.args.symlinks)
			.ignore(self.args.gitignore)
			.git_ignore(self.args.gitignore)
			.hidden(!self.args.hidden)
			.build_parallel()
			.run(|| {
				let stats = Arc::clone(&stats);
				Box::new(move |entry_result| {
					match entry_result {
						Ok(entry) if entry.file_type().is_some_and(|ft| ft.is_file()) => {
							if let Err(e) = Self::process_file_concurrent(entry.path(), &stats) {
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
		Ok(())
	}

	pub fn print_stats(&self) {
		let stats = self.stats.lock().unwrap();
		self.print_summary(&stats);
		if stats.lang_stats.is_empty() {
			println!("No recognized programming languages found.");
			return;
		}
		Self::print_language_breakdown(&stats);
	}

	fn print_summary(&self, stats: &StatsCollector) {
		println!(
			"Codestats for {}: {} {}, {} total {}, {} total size",
			self.args.path.display(),
			stats.total_files,
			utils::pluralize(stats.total_files, "file", "files"),
			stats.total_lines,
			utils::pluralize(stats.total_lines, "line", "lines"),
			human_bytes(stats.total_size as f64)
		);
		println!(
			"Line breakdown: {} code {}, {} comment {}, {} blank {}",
			stats.total_code_lines,
			utils::pluralize(stats.total_code_lines, "line", "lines"),
			stats.total_comment_lines,
			utils::pluralize(stats.total_comment_lines, "line", "lines"),
			stats.total_blank_lines,
			utils::pluralize(stats.total_blank_lines, "line", "lines")
		);
		println!(
			"Percentages: {:.1}% code, {:.1}% comments, {:.1}% blanks",
			stats.code_percentage(),
			stats.comment_percentage(),
			stats.blank_percentage()
		);
	}

	fn print_language_breakdown(stats: &StatsCollector) {
		println!("Language breakdown:");
		for (lang, lang_stats) in stats.languages_by_lines() {
			let file_pct = utils::percentage(lang_stats.files, stats.total_files);
			let line_pct = utils::percentage(lang_stats.lines, stats.total_lines);
			let size_pct = utils::percentage(lang_stats.size, stats.total_size);
			println!(
				"{lang}: {} {} ({file_pct:.1}% of files), {} {} ({line_pct:.1}% of lines), {} ({size_pct:.1}% of size)",
				lang_stats.files,
				utils::pluralize(lang_stats.files, "file", "files"),
				lang_stats.lines,
				utils::pluralize(lang_stats.lines, "line", "lines"),
				human_bytes(lang_stats.size as f64),
			);
			println!(
				"\tCode: {} lines ({:.1}%), Comments: {} lines ({:.1}%), Blank: {} lines ({:.1}%)",
				lang_stats.code_lines,
				lang_stats.code_percentage(),
				lang_stats.comment_lines,
				lang_stats.comment_percentage(),
				lang_stats.blank_lines,
				lang_stats.blank_percentage(),
			);
		}
	}

	fn process_file_concurrent(file_path: &Path, stats: &Arc<Mutex<StatsCollector>>) -> Result<()> {
		let filename = file_path.file_name().and_then(|name| name.to_str()).context("Invalid UTF-8 in file name")?;
		let language = langs::detect_language(filename)
			.with_context(|| format!("Unknown language for {}", file_path.display()))?;
		let file_size = fs::metadata(file_path)
			.with_context(|| format!("Failed to retrieve metadata for {}", file_path.display()))?
			.len();
		let (total_lines, code_lines, comment_lines, blank_lines) = Self::analyze_file_lines(file_path, &language)?;
		let file_stats = FileStats::new(total_lines, code_lines, comment_lines, blank_lines, file_size);
		stats.lock().unwrap().add_file_stats(language, file_stats);
		Ok(())
	}

	fn analyze_file_lines(file_path: &Path, language: &str) -> Result<(u64, u64, u64, u64)> {
		let file = File::open(file_path).with_context(|| format!("Failed to open file {}", file_path.display()))?;
		let reader = BufReader::new(file);
		let lang_info = langs::get_language_info(language);
		let mut line_counts = (0u64, 0u64, 0u64, 0u64); // total, code, comment, blank
		let mut comment_state = CommentState::new();
		for line_result in reader.lines() {
			let line = line_result.with_context(|| format!("Failed to read line from {}", file_path.display()))?;
			line_counts.0 += 1; // total_lines
			match comments::classify_line(&line, &lang_info, &mut comment_state) {
				LineType::Code => line_counts.1 += 1,
				LineType::Comment => line_counts.2 += 1,
				LineType::Blank => line_counts.3 += 1,
			}
		}
		Ok(line_counts)
	}
}
