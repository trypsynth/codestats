use crate::{cli::Cli, langs};
use anyhow::{Context, Result};
use human_bytes::human_bytes;
use ignore::WalkBuilder;
use std::{
    cmp::Reverse,
    collections::HashMap,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
    sync::{Arc, Mutex},
};

/// Holds statistics about a programming language's usage throughout a project/folder.
#[derive(Debug, Default, Clone)]
pub struct LangStats {
    pub files: u64,
    pub lines: u64,
    pub code_lines: u64,
    pub comment_lines: u64,
    pub blank_lines: u64,
    pub size: u64,
}

impl LangStats {
    fn add_file(&mut self, file_stats: FileStats) {
        self.files += 1;
        self.lines += file_stats.total_lines;
        self.code_lines += file_stats.code_lines;
        self.comment_lines += file_stats.comment_lines;
        self.blank_lines += file_stats.blank_lines;
        self.size += file_stats.size;
    }
}

/// Statistics for a single file
#[derive(Debug, Clone, Copy)]
struct FileStats {
    total_lines: u64,
    code_lines: u64,
    comment_lines: u64,
    blank_lines: u64,
    size: u64,
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
    fn add_file_stats(&mut self, language: String, file_stats: FileStats) {
        self.total_files += 1;
        self.total_lines += file_stats.total_lines;
        self.total_code_lines += file_stats.code_lines;
        self.total_comment_lines += file_stats.comment_lines;
        self.total_blank_lines += file_stats.blank_lines;
        self.total_size += file_stats.size;
        self.lang_stats
            .entry(language)
            .or_default()
            .add_file(file_stats);
    }
}

/// Represents different types of lines in source code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineType {
    Code,
    Comment,
    Blank,
}

/// State for tracking block comments during line analysis
#[derive(Debug, Default)]
struct CommentState {
    in_block_comment: bool,
    block_comment_depth: usize,
}

impl CommentState {
    fn enter_block(&mut self, nested: bool) {
        self.in_block_comment = true;
        if nested {
            self.block_comment_depth = 1;
        }
    }

    fn exit_block(&mut self, nested: bool) {
        if nested {
            self.block_comment_depth = self.block_comment_depth.saturating_sub(1);
            if self.block_comment_depth == 0 {
                self.in_block_comment = false;
            }
        } else {
            self.in_block_comment = false;
        }
    }

    fn enter_nested_block(&mut self) {
        self.block_comment_depth += 1;
    }
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
        stats_vec.sort_by_key(|(_, lang_stats)| Reverse(lang_stats.lines));
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
        let file_stats = FileStats {
            total_lines,
            code_lines,
            comment_lines,
            blank_lines,
            size: file_size,
        };
        stats.lock().unwrap().add_file_stats(language, file_stats);
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
        let mut comment_state = CommentState::default();
        for line in reader.lines() {
            let line = line
                .with_context(|| format!("Failed to read line from {}", file_path.display()))?;
            total_lines += 1;
            match Self::classify_line(&line, &lang_info, &mut comment_state) {
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
        comment_state: &mut CommentState,
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
                if !comment_state.in_block_comment {
                    if let Some((pos, start_len)) = Self::find_block_comment_start(line_remainder, block_comments) {
                        if pos > 0 && !line_remainder[..pos].trim().is_empty() {
                            has_code = true;
                        }
                        line_remainder = &line_remainder[pos + start_len..];
                        comment_state.enter_block(nested);
                    } else {
                        break;
                    }
                } else if let Some((pos, end_len, found_nested_start)) = Self::find_block_comment_end_or_nested_start(line_remainder, block_comments, nested) {
                    if found_nested_start {
                        comment_state.enter_nested_block();
                        line_remainder = &line_remainder[pos + end_len..];
                    } else {
                        line_remainder = &line_remainder[pos + end_len..];
                        comment_state.exit_block(nested);
                    }
                } else {
                    break;
                }
            }
        }
        if comment_state.in_block_comment {
            return if has_code { LineType::Code } else { LineType::Comment };
        }
        if let Some(ref line_comments) = lang.line_comments {
            if let Some(pos) = Self::find_line_comment_start(line_remainder, line_comments) {
                if pos > 0 && !line_remainder[..pos].trim().is_empty() {
                    has_code = true;
                }
                return if has_code { LineType::Code } else { LineType::Comment };
            }
        }
        if !line_remainder.is_empty() {
            has_code = true;
        }
        if has_code { LineType::Code } else { LineType::Comment }
    }

    fn find_block_comment_start(line: &str, block_comments: &[Vec<String>]) -> Option<(usize, usize)> {
        block_comments
            .iter()
            .filter_map(|block_pair| {
                if let [start, ..] = block_pair.as_slice() {
                    line.find(start).map(|pos| (pos, start.len()))
                } else {
                    None
                }
            })
            .min_by_key(|(pos, _)| *pos)
    }

    fn find_block_comment_end_or_nested_start(
        line: &str, 
        block_comments: &[Vec<String>], 
        nested: bool
    ) -> Option<(usize, usize, bool)> {
        for block_pair in block_comments {
            if let [start, end] = block_pair.as_slice() {
                let start_pos = if nested { line.find(start) } else { None };
                let end_pos = line.find(end);
                match (start_pos, end_pos) {
                    (Some(s_pos), Some(e_pos)) if nested && s_pos < e_pos => {
                        return Some((s_pos, start.len(), true)); // Found nested start
                    }
                    (Some(s_pos), None) if nested => {
                        return Some((s_pos, start.len(), true)); // Found nested start, no end
                    }
                    (_, Some(e_pos)) => {
                        return Some((e_pos, end.len(), false)); // Found end
                    }
                    _ => continue,
                }
            }
        }
        None
    }

    fn find_line_comment_start(line: &str, line_comments: &[String]) -> Option<usize> {
        line_comments
            .iter()
            .filter_map(|comment_start| line.find(comment_start))
            .min()
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
    fn file_stats_creation() {
        let stats = FileStats {
            total_lines: 10,
            code_lines: 8,
            comment_lines: 1,
            blank_lines: 1,
            size: 1000,
        };
        assert_eq!(stats.total_lines, 10);
        assert_eq!(stats.code_lines, 8);
    }

    #[test]
    fn lang_stats_add_file_accumulates() {
        let mut stats = LangStats::default();
        let file1 = FileStats {
            total_lines: 10,
            code_lines: 8,
            comment_lines: 1,
            blank_lines: 1,
            size: 1000,
        };
        stats.add_file(file1);
        
        assert_eq!(stats.files, 1);
        assert_eq!(stats.lines, 10);
        assert_eq!(stats.code_lines, 8);
        assert_eq!(stats.comment_lines, 1);
        assert_eq!(stats.blank_lines, 1);
        assert_eq!(stats.size, 1000);
        
        let file2 = FileStats {
            total_lines: 5,
            code_lines: 3,
            comment_lines: 2,
            blank_lines: 0,
            size: 500,
        };
        stats.add_file(file2);
        
        assert_eq!(stats.files, 2);
        assert_eq!(stats.lines, 15);
        assert_eq!(stats.code_lines, 11);
        assert_eq!(stats.comment_lines, 3);
        assert_eq!(stats.blank_lines, 1);
        assert_eq!(stats.size, 1500);
    }

    #[test]
    fn comment_state_operations() {
        let mut state = CommentState::default();
        assert!(!state.in_block_comment);
        
        state.enter_block(true);
        assert!(state.in_block_comment);
        assert_eq!(state.block_comment_depth, 1);
        
        state.enter_nested_block();
        assert_eq!(state.block_comment_depth, 2);
        
        state.exit_block(true);
        assert_eq!(state.block_comment_depth, 1);
        assert!(state.in_block_comment);
        
        state.exit_block(true);
        assert_eq!(state.block_comment_depth, 0);
        assert!(!state.in_block_comment);
    }

    #[test]
    fn classify_line_blank() {
        let mut state = CommentState::default();
        assert_eq!(
            CodeAnalyzer::classify_line("", &None, &mut state),
            LineType::Blank
        );
        assert_eq!(
            CodeAnalyzer::classify_line("   ", &None, &mut state),
            LineType::Blank
        );
    }

    #[test]
    fn classify_line_code_no_lang_info() {
        let mut state = CommentState::default();
        assert_eq!(
            CodeAnalyzer::classify_line("let x = 5;", &None, &mut state),
            LineType::Code
        );
    }
}
