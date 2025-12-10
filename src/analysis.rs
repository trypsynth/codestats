mod analyzer;
mod file_processor;
mod line_classifier;
mod stats;

pub use analyzer::CodeAnalyzer;
pub use stats::{AnalysisResults, FileStats, LanguageStats};
