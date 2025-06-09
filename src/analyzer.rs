use crate::{cli::Cli, langs};
use anyhow::{Context, Result};
use human_bytes::human_bytes;
use ignore::WalkBuilder;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
    sync::{Arc, Mutex},
};

/// Holds statistics about a programming language's usage throughout a project/folder.
#[derive(Debug, Default)]
struct LangStats {
    /// The total number of files.
    files: u64,
    /// The total number of lines.
    lines: u64,
    /// The total size (in bytes).
    size: u64,
}

impl LangStats {
    const fn add_file(&mut self, lines: u64, size: u64) {
        self.files += 1;
        self.lines += lines;
        self.size += size;
    }
}

/// Thread-safe statistics collector
#[derive(Debug, Default)]
struct StatsCollector {
    total_files: u64,
    total_lines: u64,
    total_size: u64,
    lang_stats: HashMap<String, LangStats>,
}

impl StatsCollector {
    fn add_file_stats(&mut self, language: String, lines: u64, size: u64) {
        self.total_files += 1;
        self.total_lines += lines;
        self.total_size += size;
        self.lang_stats
            .entry(language)
            .or_default()
            .add_file(lines, size);
    }
}

/// The heart of codestats, this structure performs all the analysis of a codebase/folder and prints statistics about it.
pub struct CodeAnalyzer<'a> {
    /// Holds the command-line arguments passed to the program.
    args: &'a Cli,
    /// Thread-safe statistics collector
    stats: Arc<Mutex<StatsCollector>>,
}

impl<'a> CodeAnalyzer<'a> {
    #[must_use]
    pub fn new(args: &'a Cli) -> Self {
        Self {
            args,
            stats: Arc::new(Mutex::new(StatsCollector::default())),
        }
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
                        Ok(entry) => {
                            if entry.file_type().is_some_and(|ft| ft.is_file()) {
                                if let Err(e) = Self::process_file_concurrent(entry.path(), &stats)
                                {
                                    if verbose {
                                        eprintln!(
                                            "Error processing file {}: {e}",
                                            entry.path().display()
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            if verbose {
                                eprintln!("Error walking directory: {e}");
                            }
                        }
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
        self.print_language_breakdown(&stats);
    }

    fn print_summary(&self, stats: &StatsCollector) {
        let file_word = pluralize(stats.total_files, "file", "files");
        let line_word = pluralize(stats.total_lines, "line", "lines");
        println!(
            "Codestats for {}: {} {file_word}, {} {line_word}, {} total",
            self.args.path.display(),
            stats.total_files,
            stats.total_lines,
            human_bytes(stats.total_size as f64)
        );
    }

    fn print_language_breakdown(&self, stats: &StatsCollector) {
        println!("Language breakdown:");
        let mut stats_vec: Vec<_> = stats.lang_stats.iter().collect();
        stats_vec.sort_by_key(|(lang, _)| *lang);
        for (lang, lang_stats) in stats_vec {
            let file_pct = percentage(lang_stats.files, stats.total_files);
            let line_pct = percentage(lang_stats.lines, stats.total_lines);
            let size_pct = percentage(lang_stats.size, stats.total_size);
            let file_word = pluralize(lang_stats.files, "file", "files");
            let line_word = pluralize(lang_stats.lines, "line", "lines");
            println!(
                "{lang}: {} {file_word} ({file_pct:.1}%), {} {line_word} ({line_pct:.1}%), {} ({size_pct:.1}%)",
                lang_stats.files,
                lang_stats.lines,
                human_bytes(lang_stats.size as f64),
            );
        }
    }

    fn process_file_concurrent(file_path: &Path, stats: &Arc<Mutex<StatsCollector>>) -> Result<()> {
        let filename = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .context("Invalid UTF-8 in file name")?;
        let language = langs::detect_language(filename)
            .with_context(|| format!("Unknown language for {}", file_path.display()))?;
        let metadata = fs::metadata(file_path)
            .with_context(|| format!("Failed to retrieve metadata for {}", file_path.display()))?;
        let file_size = metadata.len();
        let line_count = Self::count_lines(file_path)?;
        {
            let mut stats_guard = stats.lock().unwrap();
            stats_guard.add_file_stats(language, line_count, file_size);
        }
        Ok(())
    }

    fn count_lines(file_path: &Path) -> Result<u64> {
        let file = File::open(file_path)
            .with_context(|| format!("Failed to open file {}", file_path.display()))?;
        let reader = BufReader::new(file);
        Ok(reader.lines().count() as u64)
    }
}

fn pluralize<'a>(count: u64, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 { singular } else { plural }
}

fn percentage(part: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        (part as f64 / total as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pluralize_singular_and_plural() {
        assert_eq!(pluralize(1, "file", "files"), "file");
        assert_eq!(pluralize(0, "file", "files"), "files");
        assert_eq!(pluralize(2, "line", "lines"), "lines");
    }

    #[test]
    fn percentage_normal_cases() {
        assert_eq!(percentage(0, 100), 0.0);
        assert_eq!(percentage(1, 2), 50.0);
        assert_eq!(percentage(25, 100), 25.0);
        assert_eq!(percentage(3, 4), 75.0);
    }

    #[test]
    fn percentage_zero_total() {
        assert_eq!(percentage(10, 0), 0.0);
    }

    #[test]
    fn lang_stats_add_file_accumulates() {
        let mut stats = LangStats::default();
        stats.add_file(10, 1000);
        assert_eq!(stats.files, 1);
        assert_eq!(stats.lines, 10);
        assert_eq!(stats.size, 1000);
        stats.add_file(5, 500);
        assert_eq!(stats.files, 2);
        assert_eq!(stats.lines, 15);
        assert_eq!(stats.size, 1500);
    }

    #[test]
    fn stats_collector_add_file_stats_accumulates() {
        let mut collector = StatsCollector::default();
        collector.add_file_stats("Rust".into(), 100, 2000);
        collector.add_file_stats("Rust".into(), 200, 1000);
        collector.add_file_stats("C++".into(), 300, 500);
        assert_eq!(collector.total_files, 3);
        assert_eq!(collector.total_lines, 600);
        assert_eq!(collector.total_size, 3500);
        let rust_stats = collector.lang_stats.get("Rust").unwrap();
        assert_eq!(rust_stats.files, 2);
        assert_eq!(rust_stats.lines, 300);
        assert_eq!(rust_stats.size, 3000);
        let cpp_stats = collector.lang_stats.get("C++").unwrap();
        assert_eq!(cpp_stats.files, 1);
        assert_eq!(cpp_stats.lines, 300);
        assert_eq!(cpp_stats.size, 500);
    }

    #[test]
    fn stats_collector_handles_multiple_languages() {
        let mut collector = StatsCollector::default();
        collector.add_file_stats("Rust".into(), 10, 100);
        collector.add_file_stats("Python".into(), 20, 200);
        collector.add_file_stats("Go".into(), 30, 300);
        assert_eq!(collector.lang_stats.len(), 3);
        assert_eq!(collector.total_files, 3);
        assert_eq!(collector.total_lines, 60);
        assert_eq!(collector.total_size, 600);
    }

    #[test]
    fn stats_collector_defaults_to_empty() {
        let collector = StatsCollector::default();
        assert_eq!(collector.total_files, 0);
        assert_eq!(collector.total_lines, 0);
        assert_eq!(collector.total_size, 0);
        assert!(collector.lang_stats.is_empty());
    }

    #[test]
    fn lang_stats_default_is_zeroed() {
        let stats = LangStats::default();
        assert_eq!(stats.files, 0);
        assert_eq!(stats.lines, 0);
        assert_eq!(stats.size, 0);
    }
}
