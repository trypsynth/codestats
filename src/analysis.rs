//! Code analysis engine for processing source files and gathering statistics.
//!
//! ## Architecture
//!
//! The analysis pipeline consists of several stages:
//!
//! 1. Directory Walking ([`analyzer`]): Parallel traversal of the file tree using the `ignore` crate.
//! 2. File Processing ([`file_processor`]): For each discovered file, detect encoding, identify the programming language, filter out binary files, and choose the optimal reading strategy.
//! 3. Line Classification ([`line_classifier`]): Categorize each line as code, comment, blank, or shebang.
//! 4. Statistics Aggregation ([`stats`]): Thread-safe accumulation of per-language and per-file metrics, with optional detailed file tracking in verbose mode.

mod analyzer;
mod file_processor;
mod line_classifier;
mod stats;

pub use analyzer::CodeAnalyzer;
pub use line_classifier::LineType;
pub use stats::{AnalysisResults, FileStats, LanguageStats};
