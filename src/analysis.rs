//! The analysis pipeline consists of several stages:
//!
//! 1. Directory Walking ([`analyzer`]): Parallel walking of the file tree using the `ignore` crate.
//! 2. I/O Strategy ([`file_io`]): Chooses between buffered and memory-mapped reading based on file size.
//! 3. Encoding Detection ([`encoding`]): Detects file encoding and filters out binary files.
//! 4. Line Classification ([`line_classifier`]): Categorizes each line as code, comment, blank, or shebang.
//! 5. Line Counting ([`line_counter`]): Accumulates line statistics for each file.
//! 6. Statistics Aggregation ([`stats`]): accumulation of code stats themselves, respecting the verbose setting.

mod analyzer;
mod encoding;
mod file_io;
mod line_classifier;
mod line_counter;
mod pipeline;
pub mod stats;

pub use analyzer::CodeAnalyzer;
pub use line_classifier::LineType;
pub use stats::{AnalysisResults, FileStats, LanguageStats};
