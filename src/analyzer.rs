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
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct LangStats {
    pub files: u64,
    pub lines: u64,
    pub code_lines: u64,
    pub comment_lines: u64,
    pub blank_lines: u64,
    pub size: u64,
}

impl LangStats {
    /// Adds statistics from a single file to this language's totals
    fn add_file(&mut self, file_stats: FileStats) {
        self.files += 1;
        self.lines += file_stats.total_lines;
        self.code_lines += file_stats.code_lines;
        self.comment_lines += file_stats.comment_lines;
        self.blank_lines += file_stats.blank_lines;
        self.size += file_stats.size;
    }

    /// Returns the percentage of code lines vs total lines
    pub fn code_percentage(&self) -> f64 {
        percentage(self.code_lines, self.lines)
    }

    /// Returns the percentage of comment lines vs total lines
    pub fn comment_percentage(&self) -> f64 {
        percentage(self.comment_lines, self.lines)
    }

    /// Returns the percentage of blank lines vs total lines
    pub fn blank_percentage(&self) -> f64 {
        percentage(self.blank_lines, self.lines)
    }
}

/// Statistics for a single file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FileStats {
    total_lines: u64,
    code_lines: u64,
    comment_lines: u64,
    blank_lines: u64,
    size: u64,
}

impl FileStats {
    fn new(
        total_lines: u64,
        code_lines: u64,
        comment_lines: u64,
        blank_lines: u64,
        size: u64,
    ) -> Self {
        Self {
            total_lines,
            code_lines,
            comment_lines,
            blank_lines,
            size,
        }
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

    /// Returns languages sorted by line count (descending)
    fn languages_by_lines(&self) -> Vec<(&String, &LangStats)> {
        let mut stats_vec: Vec<_> = self.lang_stats.iter().collect();
        stats_vec.sort_by_key(|(_, lang_stats)| Reverse(lang_stats.lines));
        stats_vec
    }

    /// Returns overall code percentage
    fn code_percentage(&self) -> f64 {
        percentage(self.total_code_lines, self.total_lines)
    }

    /// Returns overall comment percentage
    fn comment_percentage(&self) -> f64 {
        percentage(self.total_comment_lines, self.total_lines)
    }

    /// Returns overall blank line percentage
    fn blank_percentage(&self) -> f64 {
        percentage(self.total_blank_lines, self.total_lines)
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
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct CommentState {
    in_block_comment: bool,
    block_comment_depth: usize,
}

impl CommentState {
    fn new() -> Self {
        Self::default()
    }

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
            self.block_comment_depth = 0;
        }
    }

    fn enter_nested_block(&mut self) {
        self.block_comment_depth += 1;
    }

    fn is_in_comment(&self) -> bool {
        self.in_block_comment
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
                        Ok(entry) if entry.file_type().map_or(false, |ft| ft.is_file()) => {
                            if let Err(e) = Self::process_file_concurrent(entry.path(), &stats) {
                                if verbose {
                                    eprintln!(
                                        "Error processing file {}: {e}",
                                        entry.path().display()
                                    );
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
        println!(
            "Percentages: {:.1}% code, {:.1}% comments, {:.1}% blank lines",
            stats.code_percentage(),
            stats.comment_percentage(),
            stats.blank_percentage()
        );
    }

    fn print_language_breakdown(stats: &StatsCollector) {
        println!("\nLanguage breakdown:");
        for (lang, lang_stats) in stats.languages_by_lines() {
            let file_pct = percentage(lang_stats.files, stats.total_files);
            let line_pct = percentage(lang_stats.lines, stats.total_lines);
            let size_pct = percentage(lang_stats.size, stats.total_size);
            println!(
                "{lang}: {} {} ({file_pct:.1}% of files), {} {} ({line_pct:.1}% of lines), {} ({size_pct:.1}% of size)",
                lang_stats.files,
                pluralize(lang_stats.files, "file", "files"),
                lang_stats.lines,
                pluralize(lang_stats.lines, "line", "lines"),
                human_bytes(lang_stats.size as f64),
            );
            println!(
                "Code: {} lines ({:.1}%), Comments: {} lines ({:.1}%), Blank: {} lines ({:.1}%)",
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
        let file_stats = FileStats::new(
            total_lines,
            code_lines,
            comment_lines,
            blank_lines,
            file_size,
        );
        stats.lock().unwrap().add_file_stats(language, file_stats);
        Ok(())
    }

    fn analyze_file_lines(file_path: &Path, language: &str) -> Result<(u64, u64, u64, u64)> {
        let file = File::open(file_path)
            .with_context(|| format!("Failed to open file {}", file_path.display()))?;
        let reader = BufReader::new(file);
        let lang_info = langs::get_language_info(language);
        let mut line_counts = (0u64, 0u64, 0u64, 0u64); // total, code, comment, blank
        let mut comment_state = CommentState::new();
        for line_result in reader.lines() {
            let line = line_result
                .with_context(|| format!("Failed to read line from {}", file_path.display()))?;
            line_counts.0 += 1; // total_lines
            match Self::classify_line(&line, &lang_info, &mut comment_state) {
                LineType::Code => line_counts.1 += 1,
                LineType::Comment => line_counts.2 += 1,
                LineType::Blank => line_counts.3 += 1,
            }
        }
        Ok(line_counts)
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
                if !comment_state.is_in_comment() {
                    if let Some((pos, start_len)) =
                        Self::find_block_comment_start(line_remainder, block_comments)
                    {
                        if pos > 0 && !line_remainder[..pos].trim().is_empty() {
                            has_code = true;
                        }
                        line_remainder = &line_remainder[pos + start_len..];
                        comment_state.enter_block(nested);
                    } else {
                        break;
                    }
                } else if let Some((pos, end_len, found_nested_start)) =
                    Self::find_block_comment_end_or_nested_start(
                        line_remainder,
                        block_comments,
                        nested,
                    )
                {
                    if found_nested_start {
                        comment_state.enter_nested_block();
                    } else {
                        comment_state.exit_block(nested);
                    }
                    line_remainder = &line_remainder[pos + end_len..];
                } else {
                    break;
                }
            }
        }
        if comment_state.is_in_comment() {
            return if has_code {
                LineType::Code
            } else {
                LineType::Comment
            };
        }
        if let Some(ref line_comments) = lang.line_comments {
            if let Some(pos) = Self::find_line_comment_start(line_remainder, line_comments) {
                if pos > 0 && !line_remainder[..pos].trim().is_empty() {
                    has_code = true;
                }
                return if has_code {
                    LineType::Code
                } else {
                    LineType::Comment
                };
            }
        }
        if !line_remainder.trim().is_empty() {
            has_code = true;
        }
        if has_code {
            LineType::Code
        } else {
            LineType::Comment
        }
    }

    fn find_block_comment_start(
        line: &str,
        block_comments: &[Vec<String>],
    ) -> Option<(usize, usize)> {
        block_comments
            .iter()
            .filter_map(|block_pair| {
                block_pair
                    .first()
                    .and_then(|start| line.find(start).map(|pos| (pos, start.len())))
            })
            .min_by_key(|(pos, _)| *pos)
    }

    fn find_block_comment_end_or_nested_start(
        line: &str,
        block_comments: &[Vec<String>],
        nested: bool,
    ) -> Option<(usize, usize, bool)> {
        for block_pair in block_comments {
            if let [start, end] = block_pair.as_slice() {
                let start_pos = if nested { line.find(start) } else { None };
                let end_pos = line.find(end);
                match (start_pos, end_pos) {
                    (Some(s_pos), Some(e_pos)) if s_pos < e_pos => {
                        return Some((s_pos, start.len(), true));
                    }
                    (Some(s_pos), None) => {
                        return Some((s_pos, start.len(), true));
                    }
                    (_, Some(e_pos)) => {
                        return Some((e_pos, end.len(), false));
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

/// Returns the singular or plural form based on count
const fn pluralize<'a>(count: u64, singular: &'a str, plural: &'a str) -> &'a str {
    if count == 1 { singular } else { plural }
}

/// Calculates percentage with zero-division safety
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
    fn test_pluralize() {
        assert_eq!(pluralize(0, "file", "files"), "files");
        assert_eq!(pluralize(1, "file", "files"), "file");
        assert_eq!(pluralize(2, "line", "lines"), "lines");
        assert_eq!(pluralize(42, "item", "items"), "items");
    }

    #[test]
    fn test_percentage() {
        assert_eq!(percentage(0, 100), 0.0);
        assert_eq!(percentage(1, 2), 50.0);
        assert_eq!(percentage(25, 100), 25.0);
        assert_eq!(percentage(3, 4), 75.0);
        assert_eq!(percentage(100, 100), 100.0);
    }

    #[test]
    fn test_percentage_zero_total() {
        assert_eq!(percentage(10, 0), 0.0);
        assert_eq!(percentage(0, 0), 0.0);
    }

    #[test]
    fn test_file_stats_creation() {
        let stats = FileStats::new(10, 8, 1, 1, 1000);
        assert_eq!(stats.total_lines, 10);
        assert_eq!(stats.code_lines, 8);
        assert_eq!(stats.comment_lines, 1);
        assert_eq!(stats.blank_lines, 1);
        assert_eq!(stats.size, 1000);
    }

    #[test]
    fn test_lang_stats_new() {
        let stats = LangStats::new();
        assert_eq!(stats.files, 0);
        assert_eq!(stats.lines, 0);
        assert_eq!(stats.code_lines, 0);
        assert_eq!(stats.comment_lines, 0);
        assert_eq!(stats.blank_lines, 0);
        assert_eq!(stats.size, 0);
    }

    #[test]
    fn test_lang_stats_add_file() {
        let mut stats = LangStats::new();
        let file1 = FileStats::new(10, 8, 1, 1, 1000);
        stats.add_file(file1);
        assert_eq!(stats.files, 1);
        assert_eq!(stats.lines, 10);
        assert_eq!(stats.code_lines, 8);
        assert_eq!(stats.comment_lines, 1);
        assert_eq!(stats.blank_lines, 1);
        assert_eq!(stats.size, 1000);
        let file2 = FileStats::new(5, 3, 2, 0, 500);
        stats.add_file(file2);
        assert_eq!(stats.files, 2);
        assert_eq!(stats.lines, 15);
        assert_eq!(stats.code_lines, 11);
        assert_eq!(stats.comment_lines, 3);
        assert_eq!(stats.blank_lines, 1);
        assert_eq!(stats.size, 1500);
    }

    #[test]
    fn test_lang_stats_percentages() {
        let mut stats = LangStats::new();
        let file_stats = FileStats::new(100, 75, 15, 10, 5000);
        stats.add_file(file_stats);
        assert_eq!(stats.code_percentage(), 75.0);
        assert_eq!(stats.comment_percentage(), 15.0);
        assert_eq!(stats.blank_percentage(), 10.0);
    }

    #[test]
    fn test_lang_stats_percentages_zero_lines() {
        let stats = LangStats::new();
        assert_eq!(stats.code_percentage(), 0.0);
        assert_eq!(stats.comment_percentage(), 0.0);
        assert_eq!(stats.blank_percentage(), 0.0);
    }

    #[test]
    fn test_comment_state_new() {
        let state = CommentState::new();
        assert!(!state.is_in_comment());
        assert_eq!(state.block_comment_depth, 0);
    }

    #[test]
    fn test_comment_state_non_nested() {
        let mut state = CommentState::new();
        state.enter_block(false);
        assert!(state.is_in_comment());
        assert_eq!(state.block_comment_depth, 0);
        state.exit_block(false);
        assert!(!state.is_in_comment());
        assert_eq!(state.block_comment_depth, 0);
    }

    #[test]
    fn test_comment_state_nested() {
        let mut state = CommentState::new();
        state.enter_block(true);
        assert!(state.is_in_comment());
        assert_eq!(state.block_comment_depth, 1);
        state.enter_nested_block();
        assert_eq!(state.block_comment_depth, 2);
        assert!(state.is_in_comment());
        state.exit_block(true);
        assert_eq!(state.block_comment_depth, 1);
        assert!(state.is_in_comment());
        state.exit_block(true);
        assert_eq!(state.block_comment_depth, 0);
        assert!(!state.is_in_comment());
    }

    #[test]
    fn test_comment_state_saturating_sub() {
        let mut state = CommentState::new();
        state.exit_block(true);
        assert_eq!(state.block_comment_depth, 0);
        assert!(!state.is_in_comment());
    }

    #[test]
    fn test_classify_line_blank() {
        let mut state = CommentState::new();
        assert_eq!(
            CodeAnalyzer::classify_line("", &None, &mut state),
            LineType::Blank
        );
        assert_eq!(
            CodeAnalyzer::classify_line("   ", &None, &mut state),
            LineType::Blank
        );
        assert_eq!(
            CodeAnalyzer::classify_line("\t\t", &None, &mut state),
            LineType::Blank
        );
        assert_eq!(
            CodeAnalyzer::classify_line(" \t \n ", &None, &mut state),
            LineType::Blank
        );
    }

    #[test]
    fn test_classify_line_no_language_info() {
        let mut state = CommentState::new();
        assert_eq!(
            CodeAnalyzer::classify_line("let x = 5;", &None, &mut state),
            LineType::Code
        );
        assert_eq!(
            CodeAnalyzer::classify_line("function test() {", &None, &mut state),
            LineType::Code
        );
    }

    #[test]
    fn test_stats_collector_accumulation() {
        let mut collector = StatsCollector::default();
        let file1 = FileStats::new(10, 8, 1, 1, 1000);
        collector.add_file_stats("Rust".to_string(), file1);
        assert_eq!(collector.total_files, 1);
        assert_eq!(collector.total_lines, 10);
        assert_eq!(collector.total_code_lines, 8);
        assert_eq!(collector.lang_stats.len(), 1);
        let file2 = FileStats::new(5, 3, 2, 0, 500);
        collector.add_file_stats("Python".to_string(), file2);
        assert_eq!(collector.total_files, 2);
        assert_eq!(collector.total_lines, 15);
        assert_eq!(collector.total_code_lines, 11);
        assert_eq!(collector.lang_stats.len(), 2);
    }

    #[test]
    fn test_stats_collector_percentages() {
        let mut collector = StatsCollector::default();
        let file_stats = FileStats::new(100, 70, 20, 10, 5000);
        collector.add_file_stats("Test".to_string(), file_stats);
        assert_eq!(collector.code_percentage(), 70.0);
        assert_eq!(collector.comment_percentage(), 20.0);
        assert_eq!(collector.blank_percentage(), 10.0);
    }

    #[test]
    fn test_stats_collector_languages_by_lines() {
        let mut collector = StatsCollector::default();
        collector.add_file_stats("Rust".to_string(), FileStats::new(100, 80, 10, 10, 2000));
        collector.add_file_stats("Python".to_string(), FileStats::new(50, 40, 5, 5, 1000));
        collector.add_file_stats(
            "JavaScript".to_string(),
            FileStats::new(75, 60, 10, 5, 1500),
        );
        let sorted = collector.languages_by_lines();
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].0, "Rust"); // 100 lines
        assert_eq!(sorted[1].0, "JavaScript"); // 75 lines
        assert_eq!(sorted[2].0, "Python"); // 50 lines
    }

    #[test]
    fn test_find_block_comment_start() {
        let block_comments = vec![vec!["/*".to_string(), "*/".to_string()]];
        assert_eq!(
            CodeAnalyzer::find_block_comment_start("/* comment */", &block_comments),
            Some((0, 2))
        );
        assert_eq!(
            CodeAnalyzer::find_block_comment_start("code /* comment", &block_comments),
            Some((5, 2))
        );
        assert_eq!(
            CodeAnalyzer::find_block_comment_start("no comment here", &block_comments),
            None
        );
    }

    #[test]
    fn test_find_line_comment_start() {
        let line_comments = vec!["//".to_string(), "#".to_string()];
        assert_eq!(
            CodeAnalyzer::find_line_comment_start("// comment", &line_comments),
            Some(0)
        );
        assert_eq!(
            CodeAnalyzer::find_line_comment_start("code // comment", &line_comments),
            Some(5)
        );
        assert_eq!(
            CodeAnalyzer::find_line_comment_start("# python comment", &line_comments),
            Some(0)
        );
        assert_eq!(
            CodeAnalyzer::find_line_comment_start("code # comment", &line_comments),
            Some(5)
        );
        assert_eq!(
            CodeAnalyzer::find_line_comment_start("no comment", &line_comments),
            None
        );
    }

    #[test]
    fn test_multiple_file_types_integration() {
        let mut collector = StatsCollector::default();
        collector.add_file_stats("Rust".to_string(), FileStats::new(200, 150, 30, 20, 8000));
        collector.add_file_stats("Python".to_string(), FileStats::new(100, 70, 20, 10, 3000));
        collector.add_file_stats("Rust".to_string(), FileStats::new(50, 40, 5, 5, 2000));
        assert_eq!(collector.total_files, 3);
        assert_eq!(collector.total_lines, 350);
        assert_eq!(collector.total_code_lines, 260);
        assert_eq!(collector.total_size, 13000);
        let rust_stats = collector.lang_stats.get("Rust").unwrap();
        assert_eq!(rust_stats.files, 2);
        assert_eq!(rust_stats.lines, 250);
        assert_eq!(rust_stats.code_lines, 190);
        let python_stats = collector.lang_stats.get("Python").unwrap();
        assert_eq!(python_stats.files, 1);
        assert_eq!(python_stats.lines, 100);
        assert_eq!(python_stats.code_lines, 70);
    }
}
