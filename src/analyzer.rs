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

/// holds statistics about a programming language's usage throughout a project/folder.
struct LangStats {
    /// The total number of files.
    files: u64,
    /// The total number of lines.
    lines: u64,
    /// The total size (in bytes).
    size: u64,
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
        CodeAnalyzer {
            args,
            total_files: 0,
            total_lines: 0,
            total_size: 0,
            lang_stats: HashMap::new(),
        }
    }

    pub fn analyze(&mut self) {
        if self.args.verbose {
            println!("Analyzing directory {}", self.args.path.display());
        }
        for result in WalkBuilder::new(&self.args.path)
            .follow_links(self.args.symlinks)
            .ignore(self.args.gitignore)
            .git_ignore(self.args.gitignore)
            .hidden(!self.args.hidden)
            .build()
        {
            if let Ok(entry) = result {
                if entry.file_type().is_some_and(|ft| ft.is_file()) {
                    if let Err(e) = self.process_file(entry.path()) {
                        if self.args.verbose {
                            eprintln!("Error processing file {}: {}", entry.path().display(), e);
                        }
                    }
                }
            } else {
                eprintln!("Error accessing entry: {:?}", result.unwrap_err());
            }
        }
    }

    pub fn print_stats(&self) {
        println!(
            "Codestats for {}: {} {}, {} {}, {} total",
            self.args.path.display(),
            self.total_files,
            if self.total_files == 1 {
                "file"
            } else {
                "files"
            },
            self.total_lines,
            if self.total_lines == 1 {
                "line"
            } else {
                "lines"
            },
            human_bytes(self.total_size as f64)
        );
        if self.lang_stats.is_empty() {
            println!("No recognized programming languages found.");
            return;
        }
        println!("Language breakdown:");
        let mut stats_vec: Vec<_> = self.lang_stats.iter().collect();
        stats_vec.sort_by_key(|(lang, _)| *lang);
        for (lang, stats) in stats_vec {
            let file_pct = (stats.files as f64 / self.total_files as f64) * 100.0;
            let line_pct = (stats.lines as f64 / self.total_lines as f64) * 100.0;
            let size_pct = (stats.size as f64 / self.total_size as f64) * 100.0;
            println!(
                "{}: {} {} ({:.1}%), {} {} ({:.1}%), {} ({:.1}%)",
                lang,
                stats.files,
                if stats.files == 1 { "file" } else { "files" },
                file_pct,
                stats.lines,
                if stats.lines == 1 { "line" } else { "lines" },
                line_pct,
                human_bytes(stats.size as f64),
                size_pct
            );
        }
    }

    fn process_file(&mut self, file_path: &Path) -> Result<()> {
        let filename = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .context("Invalid UTF-8 in file name")?;
        let language = langs::detect_language(filename)
            .ok_or_else(|| anyhow::anyhow!("Unknown language for {:?}", file_path))?;
        let metadata = fs::metadata(file_path).context("Failed to retrieve file metadata")?;
        let file_size = metadata.len();
        let file = File::open(file_path).context("Failed to open file")?;
        let reader = BufReader::new(file);
        let line_count = reader.lines().count() as u64;
        self.total_files += 1;
        self.total_lines += line_count;
        self.total_size += file_size;
        let stats = self
            .lang_stats
            .entry(language)
            .or_insert_with(|| LangStats {
                files: 0,
                lines: 0,
                size: 0,
            });
        stats.files += 1;
        stats.lines += line_count;
        stats.size += file_size;
        Ok(())
    }
}
