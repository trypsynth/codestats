use crate::cli::Cli;
use crate::langs;
use anyhow::{Result, anyhow};
use human_bytes::human_bytes;
use ignore::WalkBuilder;
use std::{
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
            match result {
                Ok(entry) if entry.file_type().is_some_and(|ft| ft.is_file()) => {
                    if let Err(e) = self.process_file(entry.path()) {
                        eprintln!("Error processing file {:?}: {}", entry.path(), e);
                    }
                }
                Ok(_) => {}
                Err(e) => eprintln!("Error accessing entry: {e}"),
            }
        }
    }

    pub fn print_stats(&self) {
        println!("Codestats for {}", self.args.path.display());
        println!("Total number of files counted: {}", self.total_files);
        println!("Total number of lines: {}", self.total_lines);
        println!("Total size: {}", human_bytes(self.total_size as f64));
    }

    fn process_file(&mut self, file_path: &Path) -> Result<()> {
        let filename = file_path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| anyhow!("Invalid UTF-8 in file name: {:?}", file_path))?;
        let language = langs::detect_language(filename)
            .ok_or_else(|| anyhow!("Unknown language for {:?}", file_path))?;
        let metadata = fs::metadata(file_path)?;
        let file_size = metadata.len();
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let line_count = reader.lines().count() as u64;
        self.total_files += 1;
        self.total_lines += line_count;
        self.total_size += file_size;
        let stats = self.lang_stats.entry(language).or_insert(LangStats {
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
