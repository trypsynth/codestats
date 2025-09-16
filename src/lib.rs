#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

//! # Codestats
//!
//! A Rust library and CLI tool for analyzing code statistics across different programming languages.
//!
//! This crate provides functionality to:
//! - Detect programming languages from file extensions and patterns
//! - Count lines of code, comments, blank lines, and shebang lines
//! - Generate detailed statistics by language and file
//! - Support for 250+ programming languages
//! - Respect `.gitignore` files and handle symbolic links
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use codestats::{AnalysisOptions, CodeAnalyzer};
//! use std::path::Path;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let options = AnalysisOptions::new("/path/to/project")
//!         .verbose(true)
//!         .respect_gitignore(true);
//!
//!     let analyzer = CodeAnalyzer::new(options);
//!     let results = analyzer.analyze()?;
//!
//!     println!("Total files: {}", results.total_files());
//!     println!("Total lines: {}", results.total_lines());
//!     println!("Code percentage: {:.1}%", results.code_percentage());
//!     Ok(())
//! }
//! ```

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
