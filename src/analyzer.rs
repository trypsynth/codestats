use crate::cli::Cli;
use crate::langs;
use anyhow::{Context, Result};
use human_bytes::human_bytes;
use ignore::WalkBuilder;
use std::{
    cmp::Reverse,
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
};

struct LangStats {
    files: u64,
    lines: u64,
    size: u64,
}

pub struct CodeAnalyzer<'a> {
    args: &'a Cli,
    total_files: u64,
    total_lines: u64,
    total_size: u64,
    lang_stats: HashMap<&'static str, LangStats>,
}

impl<'a> CodeAnalyzer<'a> {
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
            .ignore(self.args.ignores)
            .git_ignore(self.args.ignores)
            .hidden(self.args.no_hidden)
            .build()
        {
            if let Ok(entry) = result {
                if entry.file_type().map_or(false, |ft| ft.is_file()) {
                    if let Err(e) = self.process_file(entry.path()) {
                        if self.args.verbose {
                            eprintln!("Error processing file {:?}: {}", entry.path(), e);
                        }
                    }
                }
            } else {
                eprintln!("Error accessing entry: {:?}", result.unwrap_err());
            }
        }
    }

    pub fn print_stats(&self) {
        println!("Codestats for directory {}", self.args.path.display());
        println!(
            "Analyzed a total of {} {}, containing {} {} of code, with a total size of {}.",
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
            human_bytes(self.total_size as f64),
        );
        if self.lang_stats.is_empty() {
            println!("No source code languages were detected in the scanned files.");
            return;
        }
        println!("Breakdown by detected programming language:\n");
        let mut stats_vec: Vec<_> = self.lang_stats.iter().collect();
        stats_vec.sort_by_key(|(_, stat)| Reverse(stat.lines));
        for (lang, stats) in stats_vec {
            let file_pct = (stats.files as f64 / self.total_files as f64) * 100.0;
            let line_pct = (stats.lines as f64 / self.total_lines as f64) * 100.0;
            let size_pct = (stats.size as f64 / self.total_size as f64) * 100.0;
            let file_word = if stats.files == 1 { "file" } else { "files" };
            let line_word = if stats.lines == 1 { "line" } else { "lines" };
            println!(
                "- {}: {} {} ({:.1}% of all files), {} {} ({:.1}% of all lines), taking up {} ({:.1}% of total size).",
                lang,
                stats.files,
                file_word,
                file_pct,
                stats.lines,
                line_word,
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
