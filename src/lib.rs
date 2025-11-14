#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

pub mod analysis;
pub mod cli;
pub mod display;
pub mod langs;

mod utils;

pub use analysis::{
	AnalysisFlags, AnalysisOptions, AnalysisResults, CodeAnalyzer, CommentState, FileStats, LanguageStats, LineType,
	classify_line,
};
pub use display::{OutputFormat, OutputFormatter, get_formatter};
pub use langs::{detect_language, get_language_info};
