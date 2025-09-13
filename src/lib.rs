#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

pub mod analysis;
pub mod cli;
pub mod display;
pub mod language;

mod utils;

pub use analysis::{AnalysisOptions, AnalysisResults, CodeAnalyzer, FileStats, LanguageStats, LineType};
pub use display::ResultFormatter;
pub use language::detect_language;
