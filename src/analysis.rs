//! Code analysis engine for processing source files and gathering statistics.
//!
//! ## Architecture
//!
//! The analysis pipeline consists of several stages:
//!
//! 1. Directory Walking ([`analyzer`]): Parallel traversal of the file tree using the `ignore` crate.
//! 2. File Processing ([`file_processor`]): For each discovered file, orchestrates language detection and analysis.
//! 3. I/O Strategy ([`file_io`]): Chooses optimal reading strategy (buffered vs memory-mapped) based on file size.
//! 4. Encoding Detection ([`encoding`]): Detects file encoding, handles UTF-16, and filters binary files.
//! 5. Line Classification ([`line_classifier`]): Categorizes each line as code, comment, blank, or shebang.
//! 6. Line Counting ([`line_counter`]): Accumulates line statistics for each file.
//! 7. Statistics Aggregation ([`stats`]): Thread-safe accumulation of per-language and per-file metrics, with optional detailed file tracking in verbose mode.

mod analyzer;
mod encoding;
mod file_io;
mod file_processor;
mod line_classifier;
mod line_counter;
mod stats;

pub use analyzer::CodeAnalyzer;
pub use line_classifier::LineType;
pub use stats::{AnalysisResults, FileStats, LanguageStats};
