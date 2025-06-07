use crate::{cli::Cli, langs};
use anyhow::{Context, Result};
use human_bytes::human_bytes;
use ignore::WalkBuilder;
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
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
    fn add_file(&mut self, lines: u64, size: u64) {
        self.files += 1;
        self.lines += lines;
        self.size += size;
    }
}

/// The heart of codestats, this structure performs all the analysis of a codebase/folder and prints statistics about it.
pub struct CodeAnalyzer<'a> {
    /// Holds the command-line arguments passed to the program.
    args: &'a Cli,
    /// The total number of code files counted.
    total_files: u64,
    /// The total number of lines of code found.
    total_lines: u64,
    /// The total size of all the analyzed code (in bytes).
    total_size: u64,
    /// Holds per-language statistics.
    lang_stats: HashMap<String, LangStats>,
}

impl<'a> CodeAnalyzer<'a> {
    #[must_use]
    pub fn new(args: &'a Cli) -> Self {
        Self {
            args,
            total_files: 0,
            total_lines: 0,
            total_size: 0,
            lang_stats: HashMap::new(),
        }
    }

    pub fn analyze(&mut self) -> Result<()> {
        if self.args.verbose {
            println!("Analyzing directory {}", self.args.path.display());
        }
        let walker = WalkBuilder::new(&self.args.path)
            .follow_links(self.args.symlinks)
            .ignore(self.args.gitignore)
            .git_ignore(self.args.gitignore)
            .hidden(!self.args.hidden)
            .build();
        for entry in walker.flatten() {
            if entry.file_type().is_some_and(|ft| ft.is_file()) {
                if let Err(e) = self.process_file(entry.path()) {
                    if self.args.verbose {
                        eprintln!("Error processing file {}: {e}", entry.path().display());
                    }
                }
            }
        }
        Ok(())
    }

    pub fn print_stats(&self) {
        self.print_summary();
        if self.lang_stats.is_empty() {
            println!("No recognized programming languages found.");
            return;
        }
        self.print_language_breakdown();
    }

    fn print_summary(&self) {
        let file_word = pluralize(self.total_files, "file", "files");
        let line_word = pluralize(self.total_lines, "line", "lines");
        println!(
            "Codestats for {}: {} {file_word}, {} {line_word}, {} total",
            self.args.path.display(),
            self.total_files,
            self.total_lines,
            human_bytes(self.total_size as f64)
        );
    }

    fn print_language_breakdown(&self) {
        println!("Language breakdown:");
        let mut stats_vec: Vec<_> = self.lang_stats.iter().collect();
        stats_vec.sort_by_key(|(lang, _)| *lang);
        for (lang, stats) in stats_vec {
            let file_pct = percentage(stats.files, self.total_files);
            let line_pct = percentage(stats.lines, self.total_lines);
            let size_pct = percentage(stats.size, self.total_size);
            let file_word = pluralize(stats.files, "file", "files");
            let line_word = pluralize(stats.lines, "line", "lines");
            println!(
                "{lang}: {} {file_word} ({file_pct:.1}%), {} {line_word} ({line_pct:.1}%), {} ({size_pct:.1}%)",
                stats.files,
                stats.lines,
                human_bytes(stats.size as f64),
            );
        }
    }

    fn process_file(&mut self, file_path: &Path) -> Result<()> {
        let filename = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .context("Invalid UTF-8 in file name")?;
        let language = langs::detect_language(filename)
            .with_context(|| format!("Unknown language for {}", file_path.display()))?;
        let metadata = fs::metadata(file_path)
            .with_context(|| format!("Failed to retrieve metadata for {}", file_path.display()))?;
        let file_size = metadata.len();
        let line_count = self.count_lines(file_path)?;
        self.update_totals(line_count, file_size);
        self.update_language_stats(language, line_count, file_size);
        Ok(())
    }

    fn count_lines(&self, file_path: &Path) -> Result<u64> {
        let file = File::open(file_path)
            .with_context(|| format!("Failed to open file {}", file_path.display()))?;
        let reader = BufReader::new(file);
        Ok(reader.lines().count() as u64)
    }

    fn update_totals(&mut self, line_count: u64, file_size: u64) {
        self.total_files += 1;
        self.total_lines += line_count;
        self.total_size += file_size;
    }

    fn update_language_stats(&mut self, language: String, line_count: u64, file_size: u64) {
        self.lang_stats
            .entry(language)
            .or_default()
            .add_file(line_count, file_size);
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
