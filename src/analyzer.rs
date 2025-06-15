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
    files: u64,
    lines: u64,
    code_lines: u64,
    comment_lines: u64,
    blank_lines: u64,
    size: u64,
}

impl LangStats {
    fn add_file(
        &mut self,
        lines: u64,
        code_lines: u64,
        comment_lines: u64,
        blank_lines: u64,
        size: u64,
    ) {
        *self = Self {
            files: self.files + 1,
            lines: self.lines + lines,
            code_lines: self.code_lines + code_lines,
            comment_lines: self.comment_lines + comment_lines,
            blank_lines: self.blank_lines + blank_lines,
            size: self.size + size,
        };
    }
}

/// Thread-safe statistics collector
#[derive(Debug, Default)]
struct StatsCollector {
    total_files: u64,
    total_lines: u64,
    total_code_lines: u64,
    total_comment_lines: u64,
    total_blank_lines: u64,
    total_size: u64,
    lang_stats: HashMap<String, LangStats>,
}

impl StatsCollector {
    fn add_file_stats(
        &mut self,
        language: String,
        lines: u64,
        code_lines: u64,
        comment_lines: u64,
        blank_lines: u64,
        size: u64,
    ) {
        self.total_files += 1;
        self.total_lines += lines;
        self.total_code_lines += code_lines;
        self.total_comment_lines += comment_lines;
        self.total_blank_lines += blank_lines;
        self.total_size += size;
        self.lang_stats.entry(language).or_default().add_file(
            lines,
            code_lines,
            comment_lines,
            blank_lines,
            size,
        );
    }
}

/// Represents different types of lines in source code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineType {
    Code,
    Comment,
    Blank,
}

/// The heart of codestats, this structure performs all the analysis of a codebase/folder and prints statistics about it.
pub struct CodeAnalyzer<'a> {
    args: &'a Cli,
    stats: Arc<Mutex<StatsCollector>>,
}

impl<'a> CodeAnalyzer<'a> {
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
                    if let Ok(entry) = entry_result {
                        if entry.file_type().is_some_and(|ft| ft.is_file()) {
                            if let Err(e) = Self::process_file_concurrent(entry.path(), &stats) {
                                if verbose {
                                    eprintln!(
                                        "Error processing file {}: {e}",
                                        entry.path().display()
                                    );
                                }
                            }
                        }
                    } else if verbose {
                        eprintln!("Error walking directory: {}", entry_result.unwrap_err());
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
            pluralize(stats.total_files, "file", "files"),
            stats.total_lines,
            pluralize(stats.total_lines, "line", "lines"),
            human_bytes(stats.total_size as f64)
        );
        println!(
            "Line breakdown: {} code {}, {} comment {}, {} blank {}",
            stats.total_code_lines,
            pluralize(stats.total_code_lines, "line", "lines"),
            stats.total_comment_lines,
            pluralize(stats.total_comment_lines, "line", "lines"),
            stats.total_blank_lines,
            pluralize(stats.total_blank_lines, "line", "lines")
        );
        let (code_pct, comment_pct, blank_pct) = (
            percentage(stats.total_code_lines, stats.total_lines),
            percentage(stats.total_comment_lines, stats.total_lines),
            percentage(stats.total_blank_lines, stats.total_lines),
        );
        println!(
            "Percentages: {code_pct:.1}% code, {comment_pct:.1}% comments, {blank_pct:.1}% blank lines"
        );
    }

    fn print_language_breakdown(stats: &StatsCollector) {
        let mut stats_vec: Vec<_> = stats.lang_stats.iter().collect();
        stats_vec.sort_by_key(|(_, lang_stats)| std::cmp::Reverse(lang_stats.lines));
        println!("\nLanguage breakdown:");
        for (lang, lang_stats) in stats_vec {
            let (file_pct, line_pct, size_pct) = (
                percentage(lang_stats.files, stats.total_files),
                percentage(lang_stats.lines, stats.total_lines),
                percentage(lang_stats.size, stats.total_size),
            );
            println!(
                "{lang}: {} {} ({file_pct:.1}% of files), {} {} ({line_pct:.1}% of lines), {} ({size_pct:.1}% of size)",
                lang_stats.files,
                pluralize(lang_stats.files, "file", "files"),
                lang_stats.lines,
                pluralize(lang_stats.lines, "line", "lines"),
                human_bytes(lang_stats.size as f64),
            );
            let (code_pct, comment_pct, blank_pct) = (
                percentage(lang_stats.code_lines, lang_stats.lines),
                percentage(lang_stats.comment_lines, lang_stats.lines),
                percentage(lang_stats.blank_lines, lang_stats.lines),
            );
            println!(
                "Code: {} lines ({code_pct:.1}%), Comments: {} lines ({comment_pct:.1}%), Blank: {} lines ({blank_pct:.1}%)",
                lang_stats.code_lines, lang_stats.comment_lines, lang_stats.blank_lines,
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
        let file_size = fs::metadata(file_path)
            .with_context(|| format!("Failed to retrieve metadata for {}", file_path.display()))?
            .len();
        let (total_lines, code_lines, comment_lines, blank_lines) =
            Self::analyze_file_lines(file_path, &language)?;
        stats.lock().unwrap().add_file_stats(
            language,
            total_lines,
            code_lines,
            comment_lines,
            blank_lines,
            file_size,
        );
        Ok(())
    }

    fn analyze_file_lines(file_path: &Path, language: &str) -> Result<(u64, u64, u64, u64)> {
        let file = File::open(file_path)
            .with_context(|| format!("Failed to open file {}", file_path.display()))?;
        let reader = BufReader::new(file);
        let lang_info = langs::get_language_info(language);
        let mut total_lines = 0u64;
        let mut code_lines = 0u64;
        let mut comment_lines = 0u64;
        let mut blank_lines = 0u64;
        let mut in_block_comment = false;
        let mut block_comment_depth = 0;
        for line in reader.lines() {
            let line =
                line.with_context(|| format!("Failed to read line from {}", file_path.display()))?;
            total_lines += 1;
            match Self::classify_line(
                &line,
                &lang_info,
                &mut in_block_comment,
                &mut block_comment_depth,
            ) {
                LineType::Code => code_lines += 1,
                LineType::Comment => comment_lines += 1,
                LineType::Blank => blank_lines += 1,
            }
        }
        Ok((total_lines, code_lines, comment_lines, blank_lines))
    }

    fn classify_line(
        line: &str,
        lang_info: &Option<langs::Language>,
        in_block_comment: &mut bool,
        block_comment_depth: &mut usize,
    ) -> LineType {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return LineType::Blank;
        }
        let Some(lang) = lang_info else {
            return LineType::Code;
        };
        let mut line_remainder = trimmed;
        let mut has_code = false;
        if let Some(ref block_comments) = lang.block_comments {
            let nested = lang.nested_blocks.unwrap_or(false);
            while !line_remainder.is_empty() {
                if !*in_block_comment {
                    let mut found_start = false;
                    for block_pair in block_comments {
                        if let [start, ..] = block_pair.as_slice() {
                            if let Some(pos) = line_remainder.find(start) {
                                if pos > 0 && !line_remainder[..pos].trim().is_empty() {
                                    has_code = true;
                                }
                                line_remainder = &line_remainder[pos + start.len()..];
                                *in_block_comment = true;
                                if nested {
                                    *block_comment_depth = 1;
                                }
                                found_start = true;
                                break;
                            }
                        }
                    }
                    if !found_start {
                        break;
                    }
                } else {
                    let mut found_end = false;
                    for block_pair in block_comments {
                        if let [start, end] = block_pair.as_slice() {
                            if nested {
                                if let Some(start_pos) = line_remainder.find(start) {
                                    if let Some(end_pos) = line_remainder.find(end) {
                                        if start_pos < end_pos {
                                            *block_comment_depth += 1;
                                            line_remainder =
                                                &line_remainder[start_pos + start.len()..];
                                            continue;
                                        }
                                    } else {
                                        *block_comment_depth += 1;
                                        line_remainder = &line_remainder[start_pos + start.len()..];
                                        continue;
                                    }
                                }
                            }
                            if let Some(pos) = line_remainder.find(end) {
                                line_remainder = &line_remainder[pos + end.len()..];
                                if nested {
                                    *block_comment_depth = block_comment_depth.saturating_sub(1);
                                    if *block_comment_depth == 0 {
                                        *in_block_comment = false;
                                    }
                                } else {
                                    *in_block_comment = false;
                                }
                                found_end = true;
                                break;
                            }
                        }
                    }
                    if !found_end {
                        break;
                    }
                }
            }
        }
        if *in_block_comment {
            return if has_code {
                LineType::Code
            } else {
                LineType::Comment
            };
        }
        if let Some(ref line_comments) = lang.line_comments {
            for comment_start in line_comments {
                if let Some(pos) = line_remainder.find(comment_start) {
                    if pos > 0 && !line_remainder[..pos].trim().is_empty() {
                        has_code = true;
                    }
                    line_remainder = &line_remainder[pos..];
                    break;
                }
            }
        }
        if !line_remainder.is_empty() {
            if let Some(ref line_comments) = lang.line_comments {
                if line_comments
                    .iter()
                    .any(|comment_start| line_remainder.starts_with(comment_start))
                {
                    return if has_code {
                        LineType::Code
                    } else {
                        LineType::Comment
                    };
                }
            }
            has_code = true;
        }
        if has_code {
            LineType::Code
        } else {
            LineType::Comment
        }
    }
}

const fn pluralize<'a>(count: u64, singular: &'a str, plural: &'a str) -> &'a str {
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
        stats.add_file(10, 8, 1, 1, 1000);
        assert_eq!(stats.files, 1);
        assert_eq!(stats.lines, 10);
        assert_eq!(stats.code_lines, 8);
        assert_eq!(stats.comment_lines, 1);
        assert_eq!(stats.blank_lines, 1);
        assert_eq!(stats.size, 1000);
        stats.add_file(5, 3, 2, 0, 500);
        assert_eq!(stats.files, 2);
        assert_eq!(stats.lines, 15);
        assert_eq!(stats.code_lines, 11);
        assert_eq!(stats.comment_lines, 3);
        assert_eq!(stats.blank_lines, 1);
        assert_eq!(stats.size, 1500);
    }

    #[test]
    fn stats_collector_add_file_stats_accumulates() {
        let mut collector = StatsCollector::default();
        collector.add_file_stats("Rust".into(), 100, 80, 15, 5, 2000);
        collector.add_file_stats("Rust".into(), 200, 160, 30, 10, 1000);
        collector.add_file_stats("C++".into(), 300, 250, 40, 10, 500);
        assert_eq!(collector.total_files, 3);
        assert_eq!(collector.total_lines, 600);
        assert_eq!(collector.total_code_lines, 490);
        assert_eq!(collector.total_comment_lines, 85);
        assert_eq!(collector.total_blank_lines, 25);
        assert_eq!(collector.total_size, 3500);
        let rust_stats = collector.lang_stats.get("Rust").unwrap();
        assert_eq!(rust_stats.files, 2);
        assert_eq!(rust_stats.lines, 300);
        assert_eq!(rust_stats.code_lines, 240);
        assert_eq!(rust_stats.comment_lines, 45);
        assert_eq!(rust_stats.blank_lines, 15);
        assert_eq!(rust_stats.size, 3000);
    }

    #[test]
    fn classify_line_blank() {
        let mut in_block = false;
        let mut depth = 0;
        assert_eq!(
            CodeAnalyzer::classify_line("", &None, &mut in_block, &mut depth),
            LineType::Blank
        );
        assert_eq!(
            CodeAnalyzer::classify_line("   ", &None, &mut in_block, &mut depth),
            LineType::Blank
        );
    }

    #[test]
    fn classify_line_code_no_lang_info() {
        let mut in_block = false;
        let mut depth = 0;
        assert_eq!(
            CodeAnalyzer::classify_line("let x = 5;", &None, &mut in_block, &mut depth),
            LineType::Code
        );
    }
}
