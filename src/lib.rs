#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

pub mod analysis;
pub mod cli;
pub mod display;
pub mod langs;
pub mod utils;

pub use analysis::{
	AnalysisResults, AnalyzerConfig, CodeAnalyzer, CommentState, DetailLevel, FileStats, LanguageStats, LineType,
	TraversalOptions, classify_line,
};
pub use display::{OutputFormat, OutputFormatter, get_formatter};
pub use langs::{detect_language, detect_language_info, get_language_info};
